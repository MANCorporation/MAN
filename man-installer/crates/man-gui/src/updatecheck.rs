//! `man-update-check`: looks for a newer MAN on the GitHub releases of the
//! MAN repository and downloads it. Used by Settings → Updates (which shows
//! the result) and by a daily user timer (`--notify`, sends a notification).
//! Installing is left to MAN Update, which verifies the update's signature,
//! so a download from GitHub is only trusted once MAN Update accepts it.
//!
//!   man-update-check                 check, print a key file ([update] status=…)
//!   man-update-check --download V    download MAN-V.upd, print "progress F" lines, then "file PATH"
//!   man-update-check --notify        timer: check and notify once per new version
//!
//! Releases are the published GitHub releases; with Beta Updates on
//! (GSettings `beta-updates`), pre-releases count too. Those are TEST builds,
//! "MAN 10 TEST 5", with assets MAN-10-TEST-5.upd(.sha256) and a line
//! `<!-- man-base: 10.9 -->` in the release notes naming the release the
//! build was made after (see man_common::man_version_cmp).
//!
//! MAN_UPDATE_API overrides the releases API base (for testing).
use crate::i18n::{tr, trf};
use gtk::gio::prelude::*;
use gtk::{gio, glib};
use man_common::{man_version_cmp, test_build, valid_version, version_file_name, version_from_file_name, VERSION_BASE_FILE, VERSION_FILE};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const API: &str = "https://api.github.com/repos/MANCorporation/MAN";
const APP_ID: &str = "com.man.UpdateCheck";
const SCHEMA: &str = "com.man.Updates";

struct Release {
    version: String,
    /// TEST builds: the release they were made after.
    base: String,
    url: String,
    size: u64,
    sha256_url: Option<String>,
    notes: String,
    published: String,
}

enum Check {
    Live,
    Current,
    Available(Release),
}

fn current_version() -> String {
    if let Some(v) = std::fs::read_to_string(VERSION_FILE).ok().map(|s| s.trim().to_string()).filter(|s| valid_version(s)) {
        return v;
    }
    std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("VERSION_ID=").map(|v| v.trim_matches('"').to_string())))
        .filter(|v| valid_version(v))
        .unwrap_or_else(|| "10.0".into())
}

fn current_base() -> String {
    std::fs::read_to_string(VERSION_BASE_FILE).map(|s| s.trim().to_string()).ok().filter(|s| valid_version(s)).unwrap_or_default()
}

/// The live session (and MAN Setup) has the installer; installed systems do not.
fn is_live() -> bool {
    Path::new("/usr/bin/man-installer").exists()
}

fn settings() -> Option<gio::Settings> {
    gio::SettingsSchemaSource::default()?.lookup(SCHEMA, true)?;
    Some(gio::Settings::new(SCHEMA))
}

fn curl(args: &[&str]) -> Result<Vec<u8>, String> {
    let out = Command::new("curl")
        .args(["-fsSL", "--max-time", "60", "-A", "MAN-Update-Check", "-H", "Accept: application/vnd.github+json"])
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("curl: {e}"))?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn releases() -> Result<Vec<serde_json::Value>, String> {
    let api = std::env::var("MAN_UPDATE_API").unwrap_or_else(|_| API.into());
    let body = curl(&[&format!("{api}/releases?per_page=30")])?;
    let v: serde_json::Value = serde_json::from_slice(&body).map_err(|e| format!("bad reply: {e}"))?;
    Ok(v.as_array().cloned().unwrap_or_default())
}

fn beta() -> bool {
    settings().is_some_and(|s| s.boolean("beta-updates"))
}

/// Release notes without HTML comments; the `man-base:` comment, if any.
fn split_notes(body: &str) -> (String, String) {
    let (mut notes, mut base, mut rest) = (String::new(), String::new(), body.replace('\r', ""));
    while let Some(start) = rest.find("<!--") {
        notes.push_str(&rest[..start]);
        let Some(len) = rest[start..].find("-->") else { rest.clear(); break };
        if let Some(b) = rest[start + 4..start + len].trim().strip_prefix("man-base:") {
            base = b.trim().to_string();
        }
        rest = rest[start + len + 3..].to_string();
    }
    notes.push_str(&rest);
    (notes.trim().to_string(), base)
}

/// The MAN-<version>.upd assets of the published releases (and of the
/// pre-releases when `beta`).
fn updates(beta: bool) -> Result<Vec<Release>, String> {
    let mut found = vec![];
    for rel in releases()? {
        if rel["draft"].as_bool() == Some(true) || (!beta && rel["prerelease"].as_bool() == Some(true)) {
            continue;
        }
        let (notes, base) = split_notes(rel["body"].as_str().unwrap_or(""));
        let assets = rel["assets"].as_array().cloned().unwrap_or_default();
        for a in &assets {
            let name = a["name"].as_str().unwrap_or("");
            let Some(v) = name.strip_prefix("MAN-").and_then(|n| n.strip_suffix(".upd")).map(version_from_file_name) else { continue };
            let url = a["browser_download_url"].as_str().unwrap_or("");
            if !valid_version(&v) || url.is_empty() {
                continue;
            }
            let sha_name = format!("{name}.sha256");
            found.push(Release {
                base: if test_build(&v).is_some() { base.clone() } else { String::new() },
                version: v,
                url: url.to_string(),
                size: a["size"].as_u64().unwrap_or(0),
                sha256_url: assets.iter().find(|s| s["name"].as_str() == Some(&sha_name)).and_then(|s| s["browser_download_url"].as_str()).map(String::from),
                notes: notes.clone(),
                published: rel["published_at"].as_str().unwrap_or("").chars().take(10).collect(),
            });
        }
    }
    Ok(found)
}

/// The newest update that is newer than the installed system.
fn newest() -> Result<Option<Release>, String> {
    let (current, base) = (current_version(), current_base());
    Ok(updates(beta())?
        .into_iter()
        .filter(|r| man_version_cmp(&r.version, &r.base, &current, &base).is_gt())
        .max_by(|a, b| man_version_cmp(&a.version, &a.base, &b.version, &b.base)))
}

fn check() -> Result<Check, String> {
    if is_live() {
        return Ok(Check::Live);
    }
    let result = newest().map(|r| r.map_or(Check::Current, Check::Available));
    if let Some(s) = settings() {
        let now = glib::DateTime::now_utc().map(|d| d.to_unix()).unwrap_or(0);
        let _ = s.set_int64("last-check", now);
        gio::Settings::sync();
    }
    result
}

/// Key file value escaping (GKeyFile reads \n, \t and \\ back).
fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\n', "\\n").replace('\t', "\\t")
}

fn print_check() {
    let current = current_version();
    let mut out = format!("[update]\ncurrent={}\n", esc(&current));
    match check() {
        Ok(Check::Live) => out += "status=live\n",
        Ok(Check::Current) => out += "status=current\n",
        Ok(Check::Available(r)) => {
            out += &format!("status=available\nversion={}\nsize={}\npublished={}\nnotes={}\n", esc(&r.version), r.size, esc(&r.published), esc(r.notes.trim()));
            if test_build(&r.version).is_some() {
                out += "test=true\n";
            }
        }
        Err(e) => out += &format!("status=error\nerror={}\n", esc(&e)),
    }
    print!("{out}");
}

fn download_dir() -> PathBuf {
    glib::user_special_dir(glib::UserDirectory::Downloads).filter(|d| d.is_dir()).unwrap_or_else(glib::home_dir)
}

fn sha256(path: &Path) -> Option<String> {
    let out = Command::new("sha256sum").arg(path).output().ok()?;
    String::from_utf8_lossy(&out.stdout).split_whitespace().next().map(String::from)
}

fn download(version: &str) -> Result<PathBuf, String> {
    // Any channel: Settings only asks for a version it was offered.
    let release = updates(true)?.into_iter().find(|r| r.version == version).ok_or_else(|| format!("MAN {version} was not found"))?;
    let expected = match &release.sha256_url {
        Some(u) => Some(String::from_utf8_lossy(&curl(&[u])?).split_whitespace().next().unwrap_or("").to_lowercase()),
        None => None,
    };
    let dest = download_dir().join(format!("MAN-{}.upd", version_file_name(&release.version)));
    if dest.metadata().map(|m| m.len() == release.size).unwrap_or(false) && expected.as_deref().is_none_or(|e| sha256(&dest).as_deref() == Some(e)) {
        return Ok(dest);
    }
    let part = dest.with_extension("upd.part");
    let mut child = Command::new("curl")
        .args(["-fL", "--silent", "--show-error", "--retry", "3", "-C", "-", "-A", "MAN-Update-Check", "-o"])
        .arg(&part)
        .arg(&release.url)
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("curl: {e}"))?;
    let mut stdout = std::io::stdout();
    loop {
        let done = part.metadata().map(|m| m.len()).unwrap_or(0);
        if release.size > 0 {
            let _ = writeln!(stdout, "progress {:.3}", (done as f64 / release.size as f64).min(1.0));
            let _ = stdout.flush();
        }
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            if !status.success() {
                let mut err = String::new();
                if let Some(mut e) = child.stderr.take() {
                    let _ = std::io::Read::read_to_string(&mut e, &mut err);
                }
                return Err(err.trim().to_string());
            }
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    if let Some(e) = expected.filter(|e| !e.is_empty()) {
        if sha256(&part).as_deref() != Some(e.as_str()) {
            let _ = std::fs::remove_file(&part);
            return Err("the download is damaged (checksum mismatch)".into());
        }
    }
    std::fs::rename(&part, &dest).map_err(|e| e.to_string())?;
    Ok(dest)
}

fn open_updates() {
    let _ = Command::new("gnome-control-center").arg("man-updates").spawn();
}

fn app() -> gio::Application {
    let app = gio::Application::new(Some(APP_ID), gio::ApplicationFlags::default());
    let open = gio::SimpleAction::new("open-updates", None);
    open.connect_activate(|_, _| open_updates());
    app.add_action(&open);
    app.connect_activate(|_| open_updates());
    app
}

/// Timer: check, and tell the user once about each new version.
fn notify() {
    if is_live() || settings().is_some_and(|s| !s.boolean("auto-check")) {
        return;
    }
    let Ok(Check::Available(r)) = check() else { return };
    let settings = settings();
    if settings.as_ref().is_some_and(|s| s.string("offered-version") == r.version) {
        return;
    }
    let app = app();
    if app.register(gio::Cancellable::NONE).is_err() {
        return;
    }
    let n = gio::Notification::new(&trf("MAN {version} Released!", &[("version", &r.version)]));
    n.set_body(Some(tr("You can update to it.")));
    n.set_icon(&gio::ThemedIcon::new("com.man.Updater"));
    n.set_default_action("app.open-updates");
    n.add_button(tr("Open Updates"), "app.open-updates");
    app.send_notification(Some("man-update-available"), &n);
    if let Ok(bus) = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE) {
        let _ = bus.flush_sync(gio::Cancellable::NONE);
    }
    if let Some(s) = settings {
        let _ = s.set_string("offered-version", &r.version);
        gio::Settings::sync();
    }
}

pub fn main() -> glib::ExitCode {
    crate::i18n::init();
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        None | Some("--check") => print_check(),
        Some("--download") => match args.get(2).filter(|v| valid_version(v)).ok_or_else(|| "usage: --download VERSION".to_string()).and_then(|v| download(v)) {
            Ok(path) => println!("file {}", path.display()),
            Err(e) => {
                println!("error {}", e.replace('\n', " "));
                return glib::ExitCode::FAILURE;
            }
        },
        Some("--notify") => notify(),
        // D-Bus activation from a notification click.
        Some("--gapplication-service") => {
            let app = app();
            app.set_flags(gio::ApplicationFlags::IS_SERVICE);
            return app.run_with_args(&args[..1]);
        }
        Some(other) => {
            eprintln!("usage: man-update-check [--check | --download VERSION | --notify] (unknown: {other})");
            return glib::ExitCode::FAILURE;
        }
    }
    glib::ExitCode::SUCCESS
}
