//! man-app-installer-helper: the part of MAN App Installer that needs root,
//! run through pkexec (polkit action com.man.apps.install). Reads one JSON
//! request on stdin and writes `Event` lines on stdout, like the system helper:
//!
//!   {"op":"deb_install","path":"/home/ann/Downloads/app.deb"}
//!       apt-get install of a local .deb (dependencies from the apt sources)
//!   {"op":"appimage_install","path":"…/App.AppImage","id":"org.app.App","desktop":"…","icon":"…/icon.png"}
//!       AppImage for all users: moved to /opt/AppImages, icon to
//!       /opt/AppImages/.icons, menu entry /usr/local/share/applications/appimage-ID.desktop
//!       (/opt and /usr/local are kept when MAN is updated)
use man_common::Event;
use serde::Deserialize;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const APPIMAGE_DIR: &str = "/opt/AppImages";
pub const APPLICATIONS_DIR: &str = "/usr/local/share/applications";

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum Req {
    DebInstall { path: String },
    AppimageInstall { path: String, id: String, desktop: String, #[serde(default)] icon: String },
}

fn emit(ev: &Event) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{}", serde_json::to_string(ev).expect("event serializes"));
    let _ = out.flush();
}

/// Fixed details are translated by the GUI (po/extract.py collects them).
fn progress(fraction: f64, detail: &str) {
    emit(&Event::Progress { fraction, detail: detail.into() });
}

fn log(line: impl Into<String>) {
    emit(&Event::Log { line: line.into() });
}

type R = Result<String, (String, String)>;

fn fail(message: &str, detail: impl Into<String>) -> R {
    Err((message.to_string(), detail.into()))
}

/// A regular file the request may name.
fn existing_file(path: &str, ext: &str) -> Option<PathBuf> {
    let p = Path::new(path);
    (p.is_absolute() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext)) && std::fs::symlink_metadata(p).is_ok_and(|m| m.is_file())).then(|| p.to_path_buf())
}

fn deb_install(path: &str) -> R {
    let Some(file) = existing_file(path, "deb") else { return fail("The file was not found.", path) };
    // Fresh package lists, so the packages the app needs can be found
    // (offline: carry on with the lists there are).
    progress(0.0, "Getting the newest package lists…");
    let update = Command::new("apt-get").args(["update", "-q"]).env("DEBIAN_FRONTEND", "noninteractive").stdin(Stdio::null()).output();
    if let Ok(o) = &update {
        for l in String::from_utf8_lossy(&o.stdout).lines().chain(String::from_utf8_lossy(&o.stderr).lines()) {
            log(l);
        }
    }
    progress(0.0, "Installing the package…");
    let mut child = match Command::new("apt-get")
        .args(["install", "-y", "--no-install-recommends", "-o", "APT::Status-Fd=1", "-o", "Dpkg::Use-Pty=0"])
        .arg(&file)
        .env("DEBIAN_FRONTEND", "noninteractive")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return fail("The package could not be installed.", format!("apt-get: {e}")),
    };
    let mut stderr = child.stderr.take().expect("stderr");
    let errors = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr.read_to_string(&mut s);
        s
    });
    let mut tail: Vec<String> = vec![];
    for line in BufReader::new(child.stdout.take().expect("stdout")).lines().map_while(Result::ok) {
        // APT::Status-Fd: "pmstatus:PACKAGE:PERCENT:DESCRIPTION" (and dlstatus while downloading).
        let parts: Vec<&str> = line.splitn(4, ':').collect();
        if parts.len() == 4 && (parts[0] == "pmstatus" || parts[0] == "dlstatus") {
            let fraction = parts[2].parse::<f64>().unwrap_or(0.0) / 100.0;
            emit(&Event::Progress { fraction, detail: parts[3].to_string() });
        } else {
            log(&line);
            tail.push(line);
        }
    }
    let status = child.wait();
    let err = errors.join().unwrap_or_default();
    for l in err.lines() {
        log(l);
    }
    if status.is_ok_and(|s| s.success()) {
        Ok("The app was installed.".into())
    } else {
        let mut detail: Vec<&str> = err.lines().collect();
        detail.extend(tail.iter().rev().take(8).rev().map(String::as_str));
        fail("The package could not be installed.", detail.join("\n"))
    }
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 128 && !id.starts_with('.') && id.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
}

fn write(path: &Path, data: &[u8], mode: u32) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp-man");
    std::fs::write(&tmp, data)?;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(mode))?;
    std::fs::rename(&tmp, path)
}

/// Move a file, also across file systems (Downloads may be on /home).
fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    let tmp = to.with_extension("tmp-man");
    std::fs::copy(from, &tmp)?;
    std::fs::rename(&tmp, to)?;
    std::fs::remove_file(from)
}

fn appimage_install(path: &str, id: &str, desktop: &str, icon: &str) -> R {
    let Some(file) = existing_file(path, "appimage") else { return fail("The file was not found.", path) };
    if !valid_id(id) || !desktop.starts_with("[Desktop Entry]") || desktop.len() > 64 * 1024 {
        return fail("The app's menu entry is not valid.", id);
    }
    let dir = Path::new(APPIMAGE_DIR);
    let name = file.file_name().expect("file name").to_owned();
    let dest = dir.join(&name);
    let result = (|| -> std::io::Result<()> {
        std::fs::create_dir_all(dir.join(".icons"))?;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o755))?;
        std::fs::set_permissions(dir.join(".icons"), std::fs::Permissions::from_mode(0o755))?;
        log(format!("Moving {} to {}", file.display(), dest.display()));
        progress(0.2, "Copying the app");
        move_file(&file, &dest)?;
        std::os::unix::fs::chown(&dest, Some(0), Some(0))?;
        std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))?;
        progress(0.8, "Adding the app to the menu");
        let icon_src = Path::new(icon);
        if icon_src.is_absolute() && icon_src.is_file() {
            let ext = icon_src.extension().and_then(|e| e.to_str()).filter(|e| ["png", "svg", "xpm"].contains(e)).unwrap_or("png");
            write(&dir.join(".icons").join(format!("{id}.{ext}")), &std::fs::read(icon_src)?, 0o644)?;
        }
        std::fs::create_dir_all(APPLICATIONS_DIR)?;
        write(&Path::new(APPLICATIONS_DIR).join(format!("appimage-{id}.desktop")), desktop.as_bytes(), 0o644)?;
        let _ = Command::new("update-desktop-database").args(["-q", APPLICATIONS_DIR]).status();
        Ok(())
    })();
    match result {
        Ok(()) => Ok("The app was installed.".into()),
        Err(e) => fail("The app could not be installed.", e.to_string()),
    }
}

fn main() {
    let mut input = String::new();
    let _ = std::io::stdin().read_to_string(&mut input);
    let result = if !Command::new("id").arg("-u").output().is_ok_and(|o| o.stdout.starts_with(b"0")) {
        fail("Administrator rights are needed for this action.", "not running as root")
    } else {
        match serde_json::from_str::<Req>(&input) {
            Ok(Req::DebInstall { path }) => deb_install(&path),
            Ok(Req::AppimageInstall { path, id, desktop, icon }) => appimage_install(&path, &id, &desktop, &icon),
            Err(e) => fail("unknown request", e.to_string()),
        }
    };
    match result {
        Ok(message) => emit(&Event::Done { message }),
        Err((message, detail)) => {
            emit(&Event::Error { message, detail });
            std::process::exit(1);
        }
    }
}
