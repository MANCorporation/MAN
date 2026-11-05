//! The malware check of Windows programs: ClamAV plus MAN's own rules
//! (/usr/share/man/clamav). `clamscan` needs about 15 seconds to load its
//! database, so the user service `man-download-scan` checks programs as they
//! arrive in Downloads and keeps the result by SHA-256 in
//! ~/.cache/man/malware-scan: opening one is then instant. No antivirus knows
//! everything, which is why "Windows Program" warns about clean files too.
use crate::i18n::tr;
use gtk::gio::prelude::*;
use gtk::{gio, glib};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::time::{Duration, Instant, SystemTime};

const CLAM_DB: &str = "/var/lib/clamav";
const MAN_DB: &str = "/usr/share/man/clamav";
/// A clean result counts this long; the database learns new malware daily.
const CLEAN_FOR: Duration = Duration::from_secs(7 * 24 * 3600);
/// Another process is checking this file: how long its marker is believed.
const RUNNING_FOR: Duration = Duration::from_secs(600);
const EXTENSIONS: &[&str] = &["exe", "msi", "scr", "com", "bat", "cmd", "zip", "7z", "rar", "cab"];

#[derive(Debug, PartialEq, Clone)]
pub enum Scan {
    Clean,
    Flagged(String),
    /// Not fully checked: ClamAV's database is not downloaded yet, or clamscan failed.
    Failed,
}

fn cache_dir() -> PathBuf {
    glib::user_cache_dir().join("man").join("malware-scan")
}

fn sha256(path: &Path) -> Option<String> {
    let mut f = std::fs::File::open(path).ok()?;
    let mut hash = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    Some(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

fn age(path: &Path) -> Option<Duration> {
    SystemTime::now().duration_since(std::fs::metadata(path).ok()?.modified().ok()?).ok()
}

fn cached(sha: &str) -> Option<Scan> {
    let file = cache_dir().join(sha);
    let text = std::fs::read_to_string(&file).ok()?;
    match text.split_once('\n') {
        Some(("flagged", name)) => Some(Scan::Flagged(name.trim().trim_end_matches(".UNOFFICIAL").to_string())),
        Some(("clean", _)) if age(&file)? < CLEAN_FOR => Some(Scan::Clean),
        _ => None,
    }
}

fn store(sha: &str, result: &Scan) {
    let text = match result {
        Scan::Clean => "clean\n".to_string(),
        Scan::Flagged(name) => format!("flagged\n{name}\n"),
        Scan::Failed => return,
    };
    let _ = std::fs::create_dir_all(cache_dir());
    let _ = std::fs::write(cache_dir().join(sha), text);
}

/// ClamAV's two main parts are there (.cvd as downloaded, .cld once updated).
fn database_ready() -> bool {
    let has = |name: &str| ["cvd", "cld"].iter().any(|e| std::fs::metadata(format!("{CLAM_DB}/{name}.{e}")).is_ok_and(|m| m.len() > 0));
    has("main") && has("daily")
}

/// One file's verdict in clamscan's output ("PATH: OK", "PATH: NAME FOUND").
fn parse_line(output: &str, path: &str) -> Option<Scan> {
    let prefix = format!("{path}: ");
    output.lines().find_map(|l| l.strip_prefix(prefix.as_str())).and_then(|r| {
        if r == "OK" {
            Some(Scan::Clean)
        } else {
            // ClamAV marks signatures of other publishers, like MAN's own.
            r.strip_suffix(" FOUND").map(|name| Scan::Flagged(name.trim().trim_end_matches(".UNOFFICIAL").to_string()))
        }
    })
}

/// Check files with one clamscan run, as loading the database is the slow part.
/// MAN's rules work even before ClamAV's database has arrived.
fn scan_files(paths: &[PathBuf]) -> Vec<Scan> {
    let ready = database_ready();
    let mut cmd = Command::new("clamscan");
    cmd.args(["--no-summary", "--stdout", "--max-filesize=2000M", "--max-scansize=4000M", "-d", MAN_DB]);
    if ready {
        cmd.args(["-d", CLAM_DB]);
    }
    cmd.arg("--").args(paths).stdin(Stdio::null()).stderr(Stdio::null());
    let output = cmd.output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    paths
        .iter()
        .map(|p| match parse_line(&output, &p.to_string_lossy()) {
            Some(Scan::Flagged(n)) => Scan::Flagged(n),
            Some(Scan::Clean) if ready => Scan::Clean,
            _ => Scan::Failed,
        })
        .collect()
}

/// The verdict for "Windows Program": the kept one, the one being worked out
/// by the service, or a new check.
pub fn check(path: &Path) -> Scan {
    let Some(sha) = sha256(path) else { return Scan::Failed };
    if let Some(known) = cached(&sha) {
        return known;
    }
    let marker = cache_dir().join(format!("{sha}.running"));
    while age(&marker).is_some_and(|a| a < RUNNING_FOR) {
        std::thread::sleep(Duration::from_millis(200));
        if let Some(known) = cached(&sha) {
            return known;
        }
    }
    let result = scan_files(&[path.to_path_buf()]).remove(0);
    store(&sha, &result);
    result
}

// ─────────────────────────── The service ───────────────────────────

/// Check what is not known yet; returns the newly flagged files (known ones
/// were reported when they were found).
fn scan_batch(paths: Vec<PathBuf>) -> Vec<(PathBuf, String)> {
    let _ = std::fs::create_dir_all(cache_dir());
    let mut todo = vec![];
    let mut flagged = vec![];
    for p in paths {
        let Some(sha) = sha256(&p) else { continue };
        match cached(&sha) {
            Some(_) => {}
            None => {
                let _ = std::fs::write(cache_dir().join(format!("{sha}.running")), "");
                todo.push((p, sha));
            }
        }
    }
    if todo.is_empty() {
        return flagged;
    }
    let files: Vec<PathBuf> = todo.iter().map(|(p, _)| p.clone()).collect();
    for ((p, sha), result) in todo.into_iter().zip(scan_files(&files)) {
        store(&sha, &result);
        let _ = std::fs::remove_file(cache_dir().join(format!("{sha}.running")));
        if let Scan::Flagged(name) = result {
            flagged.push((p, name));
        }
    }
    flagged
}

fn is_candidate(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| EXTENSIONS.contains(&e.to_lowercase().as_str()))
}

#[derive(Default)]
struct Watch {
    monitors: HashMap<PathBuf, gio::FileMonitor>,
    /// File → when it last changed: checked once it has been quiet for a moment.
    pending: HashMap<PathBuf, Instant>,
    busy: bool,
}

fn add_dir(watch: &Rc<RefCell<Watch>>, dir: &Path, depth: u32) {
    if depth > 6 || watch.borrow().monitors.contains_key(dir) {
        return;
    }
    let Ok(monitor) = gio::File::for_path(dir).monitor_directory(gio::FileMonitorFlags::WATCH_MOVES, gio::Cancellable::NONE) else { return };
    let w = watch.clone();
    monitor.connect_changed(move |_, file, other, event| {
        use gio::FileMonitorEvent as E;
        let target = match event {
            E::Created | E::Changed | E::ChangesDoneHint | E::MovedIn => Some(file.clone()),
            E::Renamed => other.cloned(),
            _ => None,
        };
        let Some(path) = target.and_then(|f| f.path()) else { return };
        note(&w, &path, depth + 1);
    });
    watch.borrow_mut().monitors.insert(dir.to_path_buf(), monitor);
    // Folders that appear in one go (an unpacked archive) have files already.
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                add_dir(watch, &path, depth + 1);
            } else if is_candidate(&path) {
                watch.borrow_mut().pending.insert(path, Instant::now());
            }
        }
    }
}

fn note(watch: &Rc<RefCell<Watch>>, path: &Path, depth: u32) {
    if path.is_dir() {
        add_dir(watch, path, depth);
    } else if is_candidate(path) {
        watch.borrow_mut().pending.insert(path.to_path_buf(), Instant::now());
    }
}

fn notify(app: &gio::Application, path: &Path) {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let n = gio::Notification::new(tr("MAN Flagged This Program as Malware"));
    n.set_body(Some(&name));
    n.set_priority(gio::NotificationPriority::High);
    n.set_icon(&gio::ThemedIcon::new("man-alert-danger"));
    n.set_default_action_and_target_value("app.open-file", Some(&path.to_string_lossy().to_variant()));
    app.send_notification(None, &n);
}

fn tick(app: &gio::Application, watch: &Rc<RefCell<Watch>>) {
    let due: Vec<PathBuf> = {
        let mut w = watch.borrow_mut();
        if w.busy {
            return;
        }
        let due: Vec<PathBuf> = w.pending.iter().filter(|(_, t)| t.elapsed() > Duration::from_secs(3)).map(|(p, _)| p.clone()).collect();
        for p in &due {
            w.pending.remove(p);
        }
        w.busy = !due.is_empty();
        due
    };
    if due.is_empty() {
        return;
    }
    let (app, watch) = (app.clone(), watch.clone());
    glib::spawn_future_local(async move {
        let flagged = gio::spawn_blocking(move || scan_batch(due.into_iter().filter(|p| p.is_file()).collect())).await.unwrap_or_default();
        for (path, _) in flagged {
            notify(&app, &path);
        }
        watch.borrow_mut().busy = false;
    });
}

pub fn service_main() -> glib::ExitCode {
    crate::i18n::init();
    let app = gio::Application::new(Some("com.man.DownloadScan"), gio::ApplicationFlags::empty());
    let open = gio::SimpleAction::new("open-file", Some(glib::VariantTy::STRING));
    open.connect_activate(|_, target| {
        if let Some(path) = target.and_then(|v| v.get::<String>()) {
            let _ = Command::new("man-windows-program").arg(path).spawn();
        }
    });
    app.add_action(&open);
    let watch: Rc<RefCell<Watch>> = Rc::default();
    app.connect_activate(move |app| {
        // Keeps the service running: it has no windows.
        std::mem::forget(app.hold());
        let downloads = glib::user_special_dir(glib::UserDirectory::Downloads).unwrap_or_else(|| glib::home_dir().join("Downloads"));
        let _ = std::fs::create_dir_all(&downloads);
        add_dir(&watch, &downloads, 0);
        let (app, watch) = (app.clone(), watch.clone());
        glib::timeout_add_seconds_local(2, move || {
            tick(&app, &watch);
            glib::ControlFlow::Continue
        });
    });
    app.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamscan_lines() {
        let out = "/a b/x.exe: OK\n/a b/y: z.exe: Win.Trojan.Agent-1 FOUND\n";
        assert_eq!(parse_line(out, "/a b/x.exe"), Some(Scan::Clean));
        assert_eq!(parse_line(out, "/a b/y: z.exe"), Some(Scan::Flagged("Win.Trojan.Agent-1".into())));
        assert_eq!(parse_line(out, "/other"), None);
        assert_eq!(parse_line("/m.exe: MAN.Trojan.Memz.UNOFFICIAL FOUND\n", "/m.exe"), Some(Scan::Flagged("MAN.Trojan.Memz".into())));
        assert_eq!(parse_line("/x: Can't open file ERROR\n", "/x"), None);
    }

    #[test]
    fn candidates() {
        assert!(is_candidate(Path::new("/d/SETUP.EXE")));
        assert!(is_candidate(Path::new("/d/a.zip")));
        assert!(!is_candidate(Path::new("/d/a.exe.part")));
        assert!(!is_candidate(Path::new("/d/photo.jpg")));
    }
}
