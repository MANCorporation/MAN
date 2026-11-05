//! "Windows Program": what opens when a Windows program (.exe, .msi) is
//! double-clicked. It checks the file with ClamAV and always warns before Wine
//! runs it:
//!
//! - nothing found (or the check was not possible): a warning, Run It / Cancel;
//! - malware found: Move to Trash / Cancel / Bypass and Run, and bypassing needs
//!   an administrator's password and one more confirmation.
//!
//! Wine does not contain a program: once running, it can reach everything the
//! user can. ClamAV only knows known malware, so the warning stays either way.
use crate::i18n::{tr, trf};
use adw::prelude::*;
use gtk::{gio, glib};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const APP_ID: &str = "com.man.WindowsProgram";
/// Exits 0 after polkit asked for the administrator's password.
const BYPASS_HELPER: &str = "/usr/libexec/man-installer/man-windows-bypass";

// ─────────────────────────── The check ─────────────────────────────

#[derive(Debug, PartialEq)]
enum Scan {
    Clean,
    Flagged(String),
    /// ClamAV could not check the file (e.g. its database is not downloaded yet).
    Failed,
}

#[derive(Debug, PartialEq)]
enum Format {
    Runs,
    /// Made for this name of a processor, not this computer's.
    OtherProcessor(&'static str),
    NotWindows,
}

/// The machine type in the PE header of an .exe. .msi files are not PE files.
fn pe_machine(path: &Path) -> Option<u16> {
    let mut f = std::fs::File::open(path).ok()?;
    let mut mz = [0u8; 64];
    f.read_exact(&mut mz).ok()?;
    if &mz[..2] != b"MZ" {
        return None;
    }
    let pe = u32::from_le_bytes(mz[0x3c..0x40].try_into().ok()?) as u64;
    f.seek(SeekFrom::Start(pe)).ok()?;
    let mut head = [0u8; 6];
    f.read_exact(&mut head).ok()?;
    (&head[..4] == b"PE\0\0").then(|| u16::from_le_bytes([head[4], head[5]]))
}

fn format_of(machine: Option<u16>) -> Format {
    const X86: u16 = 0x14c;
    const X64: u16 = 0x8664;
    const ARM64: u16 = 0xaa64;
    const ARM64EC: u16 = 0xa641;
    const ARMNT: u16 = 0x1c4;
    match machine {
        None => Format::NotWindows,
        Some(X86 | X64) if cfg!(target_arch = "aarch64") => Format::OtherProcessor("Intel/AMD"),
        Some(ARM64 | ARM64EC | ARMNT) if !cfg!(target_arch = "aarch64") => Format::OtherProcessor("ARM"),
        Some(_) => Format::Runs,
    }
}

fn is_msi(path: &Path) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("msi"))
}

/// What clamscan's exit code and output mean: 0 clean, 1 found, else error.
fn parse_scan(code: Option<i32>, stdout: &str) -> Scan {
    match code {
        Some(0) => Scan::Clean,
        Some(1) => {
            let name = stdout.lines().find_map(|l| l.strip_suffix(" FOUND")).and_then(|l| l.rsplit(": ").next()).unwrap_or("").trim();
            Scan::Flagged(name.to_string())
        }
        _ => Scan::Failed,
    }
}

fn scan(path: &Path) -> Scan {
    // Installers are large archives: look inside them, not just at their start.
    let out = Command::new("clamscan")
        .args(["--no-summary", "--infected", "--stdout", "--max-filesize=2000M", "--max-scansize=4000M", "--"])
        .arg(path)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output();
    match out {
        Ok(o) => parse_scan(o.status.code(), &String::from_utf8_lossy(&o.stdout)),
        Err(_) => Scan::Failed,
    }
}

// ─────────────────────────── Running it ────────────────────────────

fn wine_prefix() -> PathBuf {
    std::env::var_os("WINEPREFIX").map(PathBuf::from).unwrap_or_else(|| glib::home_dir().join(".wine"))
}

/// First use: Wine's folders. Programs must not take over opening the user's
/// files, so installers' file associations stay inside Wine.
fn prepare_wine() -> Result<(), String> {
    let prefix = wine_prefix();
    let marker = prefix.join(".man-prepared");
    if marker.exists() {
        return Ok(());
    }
    let run = |args: &[&str]| {
        Command::new(args[0])
            .args(&args[1..])
            .env("WINEDEBUG", "-all")
            // Asks to download .NET and web components otherwise, before anything is shown.
            .env("WINEDLLOVERRIDES", "mscoree,mshtml=")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    };
    if !run(&["wineboot", "-i"]) {
        return Err(tr("Windows support could not be started.").to_string());
    }
    run(&["wine", "reg", "add", r"HKCU\Software\Wine\FileOpenAssociations", "/v", "Enable", "/t", "REG_SZ", "/d", "N", "/f"]);
    // Wine 10 ignores the position GNOME gives a window when it adds its title
    // bar, and then tests mouse clicks about 38 pixels off: buttons do not react.
    // With Wine's own title bar the positions agree.
    run(&["wine", "reg", "add", r"HKCU\Software\Wine\X11 Driver", "/v", "Decorated", "/t", "REG_SZ", "/d", "N", "/f"]);
    std::fs::write(marker, "").map_err(|e| e.to_string())
}

fn start_program(path: &Path) -> Result<(), String> {
    use std::os::unix::process::CommandExt;
    let mut cmd = Command::new("wine");
    if is_msi(path) {
        cmd.args(["msiexec", "/i"]);
    }
    cmd.arg(path).env("WINEDEBUG", "-all").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).process_group(0);
    if let Some(dir) = path.parent() {
        cmd.current_dir(dir);
    }
    cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
}

// ─────────────────────────── The window ────────────────────────────

fn status(icon: &str, title: &str, description: &str) -> adw::StatusPage {
    let page = adw::StatusPage::builder().icon_name(icon).title(title).description(glib::markup_escape_text(description).as_str()).build();
    page.add_css_class("compact");
    page
}

fn pill(label: &str, class: Option<&str>) -> gtk::Button {
    let b = gtk::Button::builder().label(label).use_underline(true).halign(gtk::Align::Center).width_request(240).build();
    b.add_css_class("pill");
    if let Some(c) = class {
        b.add_css_class(c);
    }
    b
}

fn buttons(page: &adw::StatusPage, list: &[&gtk::Button]) {
    let b = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(10).halign(gtk::Align::Center).build();
    for button in list {
        b.append(*button);
    }
    page.set_child(Some(&b));
}

fn show(toast: &adw::ToastOverlay, page: &impl IsA<gtk::Widget>) {
    toast.set_child(Some(page));
}

fn window_of(toast: &adw::ToastOverlay) -> Option<gtk::Window> {
    toast.root().and_downcast::<gtk::Window>()
}

fn close(toast: &adw::ToastOverlay) {
    if let Some(w) = window_of(toast) {
        w.close();
    }
}

/// Run the program in Wine, preparing Wine the first time, then close.
fn run_program(toast: &adw::ToastOverlay, path: PathBuf) {
    let page = status("emblem-system-symbolic", tr("Starting the Windows Program"), tr("The first start takes a moment."));
    page.set_child(Some(&adw::Spinner::builder().width_request(32).height_request(32).halign(gtk::Align::Center).build()));
    show(toast, &page);
    let toast = toast.clone();
    glib::spawn_future_local(async move {
        let result = gio::spawn_blocking(move || prepare_wine().and_then(|_| start_program(&path))).await.unwrap_or_else(|_| Err("crashed".into()));
        match result {
            Ok(()) => close(&toast),
            Err(e) => {
                let page = status("dialog-error-symbolic", tr("The Windows Program Could Not Start"), &e);
                let done = pill(tr("_Close"), None);
                let t = toast.clone();
                done.connect_clicked(move |_| close(&t));
                buttons(&page, &[&done]);
                show(&toast, &page);
            }
        }
    });
}

/// The normal case: nothing known was found, which does not make it safe.
fn warn_page(toast: &adw::ToastOverlay, path: PathBuf, checked: bool) {
    let mut text = tr("Running Windows programs (.exe) is not recommended, as they can likely contain malware or things that can harm your privacy.").to_string();
    if !checked {
        text.push_str("\n\n");
        text.push_str(tr("MAN could not check this program for malware: the malware database is not ready yet. Connect to the internet and try again in a few minutes."));
    }
    let page = status("dialog-warning-symbolic", tr("Run This Windows Program?"), &text);
    page.set_tooltip_text(path.file_name().and_then(|n| n.to_str()));
    let run = pill(tr("_Run It"), None);
    let cancel = pill(tr("_Cancel"), Some("suggested-action"));
    {
        let (t, p) = (toast.clone(), path);
        run.connect_clicked(move |_| run_program(&t, p.clone()));
    }
    {
        let t = toast.clone();
        cancel.connect_clicked(move |_| close(&t));
    }
    buttons(&page, &[&cancel, &run]);
    show(toast, &page);
    cancel.grab_focus();
}

/// The last question after the password: Cancel stays the default.
fn confirm_bypass(toast: &adw::ToastOverlay, path: PathBuf) {
    let dialog = adw::AlertDialog::new(
        Some(tr("Run a Program Flagged as Malware?")),
        Some(tr("This is your last chance to stop. MAN found malware in this program, and Windows programs are not isolated from your files. It could:\n\n• steal your passwords, documents and photos\n• delete your files or hold them for ransom\n• spy on what you do and keep running in the background\n\nMAN cannot undo what the program does. Only continue if you know exactly why this program was flagged.")),
    );
    dialog.add_response("cancel", tr("_Cancel"));
    dialog.add_response("run", tr("Run _Anyway"));
    dialog.set_response_appearance("run", adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    let toast = toast.clone();
    glib::spawn_future_local(async move {
        if dialog.choose_future(window_of(&toast).as_ref()).await == "run" {
            run_program(&toast, path);
        }
    });
}

fn flagged_page(toast: &adw::ToastOverlay, path: PathBuf, name: &str) {
    let mut text = tr("MAN flagged this Windows program (.exe) as malware, which can harm your data or privacy. It should not be run and should be moved to the trash.").to_string();
    if !name.is_empty() {
        text.push_str("\n\n");
        text.push_str(&trf("Detected as: {name}", &[("name", &name)]));
    }
    let page = status("dialog-error-symbolic", tr("MAN Flagged This Program as Malware"), &text);
    let trash = pill(tr("_Move to Trash"), Some("suggested-action"));
    let cancel = pill(tr("_Cancel"), None);
    let bypass = pill(tr("_Bypass and Run"), Some("destructive-action"));
    {
        let (t, p) = (toast.clone(), path.clone());
        trash.connect_clicked(move |_| match gio::File::for_path(&p).trash(gio::Cancellable::NONE) {
            Ok(()) => close(&t),
            Err(e) => t.add_toast(adw::Toast::new(&e.to_string())),
        });
    }
    {
        let t = toast.clone();
        cancel.connect_clicked(move |_| close(&t));
    }
    {
        let (t, p) = (toast.clone(), path);
        bypass.connect_clicked(move |b| {
            let (t, p, b) = (t.clone(), p.clone(), b.clone());
            b.set_sensitive(false);
            glib::spawn_future_local(async move {
                let allowed = gio::spawn_blocking(|| Command::new("pkexec").arg(BYPASS_HELPER).stdin(Stdio::null()).status().map(|s| s.success()).unwrap_or(false)).await.unwrap_or(false);
                b.set_sensitive(true);
                if allowed {
                    confirm_bypass(&t, p);
                }
            });
        });
    }
    buttons(&page, &[&trash, &cancel, &bypass]);
    show(toast, &page);
    trash.grab_focus();
}

fn open_file(app: &adw::Application, path: PathBuf) {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let title = adw::WindowTitle::new(tr("Windows Program"), &name);
    let header = adw::HeaderBar::builder().title_widget(&title).build();
    let toast = adw::ToastOverlay::new();
    let tv = adw::ToolbarView::new();
    tv.add_top_bar(&header);
    tv.set_content(Some(&toast));
    let window = adw::ApplicationWindow::builder().application(app).default_width(560).default_height(640).content(&tv).icon_name(APP_ID).title(tr("Windows Program")).build();

    let msi = is_msi(&path);
    let format = if msi { Format::Runs } else { format_of(pe_machine(&path)) };
    let refuse = |heading: &str, text: String| {
        let page = status("dialog-error-symbolic", heading, &text);
        let done = pill(tr("_Close"), Some("suggested-action"));
        let t = toast.clone();
        done.connect_clicked(move |_| close(&t));
        buttons(&page, &[&done]);
        show(&toast, &page);
    };
    match format {
        Format::NotWindows => refuse(tr("This Is Not a Windows Program"), tr("This file cannot be run.").to_string()),
        Format::OtherProcessor(arch) => refuse(tr("This Program Cannot Run on This Computer"), trf("It was made for {arch} processors.", &[("arch", &arch)])),
        Format::Runs => {
            let page = status("system-search-symbolic", tr("Checking This Program for Malware"), &name);
            page.set_child(Some(&adw::Spinner::builder().width_request(32).height_request(32).halign(gtk::Align::Center).build()));
            show(&toast, &page);
            let t = toast.clone();
            glib::spawn_future_local(async move {
                let p = path.clone();
                let result = gio::spawn_blocking(move || scan(&p)).await.unwrap_or(Scan::Failed);
                match result {
                    Scan::Clean => warn_page(&t, path, true),
                    Scan::Failed => warn_page(&t, path, false),
                    Scan::Flagged(name) => flagged_page(&t, path, &name),
                }
            });
        }
    }
    window.present();
}

pub fn main() -> glib::ExitCode {
    crate::i18n::init();
    let app = adw::Application::builder().application_id(APP_ID).flags(gio::ApplicationFlags::HANDLES_OPEN | gio::ApplicationFlags::NON_UNIQUE).build();
    app.connect_open(|app, files, _| {
        for file in files {
            if let Some(path) = file.path() {
                open_file(app, path);
            }
        }
    });
    app.connect_activate(|_| eprintln!("usage: man-windows-program FILE.exe"));
    app.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamscan_results() {
        assert_eq!(parse_scan(Some(0), ""), Scan::Clean);
        assert_eq!(parse_scan(Some(1), "/home/a/x.exe: Win.Test.EICAR_HDB-1 FOUND\n"), Scan::Flagged("Win.Test.EICAR_HDB-1".into()));
        assert_eq!(parse_scan(Some(1), "/home/a: b/x y.exe: Win.Trojan.Agent-1 FOUND\n"), Scan::Flagged("Win.Trojan.Agent-1".into()));
        assert_eq!(parse_scan(Some(2), ""), Scan::Failed);
        assert_eq!(parse_scan(None, ""), Scan::Failed);
    }

    #[test]
    fn processors() {
        assert_eq!(format_of(None), Format::NotWindows);
        if cfg!(target_arch = "aarch64") {
            assert_eq!(format_of(Some(0x8664)), Format::OtherProcessor("Intel/AMD"));
            assert_eq!(format_of(Some(0xaa64)), Format::Runs);
        } else {
            assert_eq!(format_of(Some(0x8664)), Format::Runs);
            assert_eq!(format_of(Some(0x14c)), Format::Runs);
            assert_eq!(format_of(Some(0xaa64)), Format::OtherProcessor("ARM"));
        }
    }
}
