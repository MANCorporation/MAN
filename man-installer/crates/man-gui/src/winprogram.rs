//! "Windows Program": what opens when a Windows program (.exe, .msi) is
//! double-clicked. It takes the malware check's verdict (`malscan`, usually
//! ready since the download) and always warns before Wine runs it, in a small
//! popup laid out like macOS alerts:
//!
//! - nothing found (or the check was not possible): a warning, Run It / Cancel;
//! - malware found: Move to Trash / Cancel / Bypass and Run, and bypassing needs
//!   an administrator's password and one more confirmation.
//!
//! Wine does not contain a program: once running, it can reach everything the
//! user can. Scanners only know known malware, so the warning stays either way.
use crate::i18n::{tr, trf};
use crate::malscan::{self, Scan};
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

// ─────────────────────────── The popup ─────────────────────────────

const CSS: &str = r#"
.man-alert { background-color: var(--dialog-bg-color); }
.man-alert-title { font-size: 13pt; font-weight: 700; }
.man-alert-name { font-size: 9pt; opacity: 0.65; }
.man-alert-body { font-size: 10.5pt; }
.man-alert-button { min-height: 30px; border-radius: 8px; font-weight: 500; background: alpha(currentColor, 0.12); }
.man-alert-button:hover { background: alpha(currentColor, 0.18); }
.man-alert-button.default { background: var(--accent-bg-color); color: var(--accent-fg-color); }
.man-alert-button.default:hover { background: color-mix(in srgb, var(--accent-bg-color) 88%, white); }
.man-alert-button.danger { color: var(--destructive-color); }
"#;

const WIDTH: i32 = 300;

pub(crate) type Btn = (&'static str, &'static str, Box<dyn Fn(&gtk::Window, &gtk::Button)>);

/// The popup's content: icon, bold title, file name, text, then the buttons,
/// side by side when there are two and stacked when there are more. The first
/// button is the default one (blue, answers Enter). Two buttons show it on the right.
pub(crate) fn show(window: &gtk::Window, icon: &str, title: &str, body: &str, buttons: Vec<Btn>, spinner: bool) {
    let page = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(10).margin_top(22).margin_bottom(18).margin_start(20).margin_end(20).width_request(WIDTH).build();
    let image = gtk::Image::from_icon_name(icon);
    image.set_pixel_size(72);
    page.append(&image);
    let heading = gtk::Label::builder().label(title).wrap(true).justify(gtk::Justification::Center).max_width_chars(30).margin_top(4).build();
    heading.add_css_class("man-alert-title");
    page.append(&heading);
    if let Some(name) = window.widget_name().strip_prefix("file:").filter(|n| !n.is_empty()) {
        let label = gtk::Label::builder().label(name).ellipsize(gtk::pango::EllipsizeMode::Middle).max_width_chars(34).build();
        label.add_css_class("man-alert-name");
        page.append(&label);
    }
    if !body.is_empty() {
        let text = gtk::Label::builder().label(body).wrap(true).wrap_mode(gtk::pango::WrapMode::WordChar).justify(gtk::Justification::Center).max_width_chars(36).build();
        text.add_css_class("man-alert-body");
        page.append(&text);
    }
    if spinner {
        page.append(&adw::Spinner::builder().width_request(24).height_request(24).halign(gtk::Align::Center).margin_top(6).build());
    }
    let side_by_side = buttons.len() == 2;
    let row = gtk::Box::builder().orientation(if side_by_side { gtk::Orientation::Horizontal } else { gtk::Orientation::Vertical }).spacing(10).homogeneous(side_by_side).margin_top(10).build();
    let mut default = None;
    let mut made = vec![];
    for (i, (label, class, action)) in buttons.into_iter().enumerate() {
        let b = gtk::Button::builder().label(label).use_underline(true).hexpand(true).build();
        b.add_css_class("flat");
        b.add_css_class("man-alert-button");
        if !class.is_empty() {
            b.add_css_class(class);
        }
        if i == 0 {
            b.add_css_class("default");
            default = Some(b.clone());
        }
        let w = window.clone();
        b.connect_clicked(move |b| action(&w, b));
        made.push(b);
    }
    if side_by_side {
        made.reverse();
    }
    let has_buttons = !made.is_empty();
    for b in &made {
        row.append(b);
    }
    if has_buttons {
        page.append(&row);
    }
    let handle = gtk::WindowHandle::builder().child(&page).build();
    if let Some(w) = window.downcast_ref::<adw::Window>() {
        w.set_content(Some(&handle));
    }
    window.set_default_widget(default.as_ref());
    if let Some(b) = default {
        b.grab_focus();
    }
}

fn cancel_button(label: &'static str) -> Btn {
    (label, "", Box::new(|w, _| w.close()))
}

fn show_error(window: &gtk::Window, title: &str, text: &str) {
    show(window, "man-alert-danger", title, text, vec![cancel_button(tr("_Close"))], false);
}

/// Run the program in Wine, preparing Wine the first time, then close.
fn run_program(window: &gtk::Window, path: PathBuf) {
    show(window, "man-alert-warning", tr("Starting the Windows Program"), tr("The first start takes a moment."), vec![], true);
    let window = window.clone();
    glib::spawn_future_local(async move {
        let result = gio::spawn_blocking(move || prepare_wine().and_then(|_| start_program(&path))).await.unwrap_or_else(|_| Err("crashed".into()));
        match result {
            Ok(()) => window.close(),
            Err(e) => show_error(&window, tr("The Windows Program Could Not Start"), &e),
        }
    });
}

/// The normal case: nothing known was found, which does not make it safe.
fn warn_page(window: &gtk::Window, path: PathBuf, checked: bool) {
    let mut text = tr("Running Windows programs (.exe) is not recommended, as they can likely contain malware or things that can harm your privacy.").to_string();
    if !checked {
        text.push_str("\n\n");
        text.push_str(tr("MAN could not check this program for malware: the malware database is not ready yet. Connect to the internet and try again in a few minutes."));
    }
    let buttons: Vec<Btn> = vec![cancel_button(tr("_Cancel")), (tr("_Run It"), "", Box::new(move |w, _| run_program(w, path.clone())))];
    show(window, "man-alert-warning", tr("Run This Windows Program?"), &text, buttons, false);
}

/// The last question after the password: Cancel stays the default.
fn confirm_bypass(window: &gtk::Window, path: PathBuf) {
    let buttons: Vec<Btn> = vec![cancel_button(tr("_Cancel")), (tr("Run _Anyway"), "danger", Box::new(move |w, _| run_program(w, path.clone())))];
    show(
        window,
        "man-alert-danger",
        tr("Run a Program Flagged as Malware?"),
        tr("This is your last chance to stop. MAN found malware in this program, and Windows programs are not isolated from your files. It could:\n\n• steal your passwords, documents and photos\n• delete your files or hold them for ransom\n• spy on what you do and keep running in the background\n\nMAN cannot undo what the program does. Only continue if you know exactly why this program was flagged."),
        buttons,
        false,
    );
}

fn flagged_page(window: &gtk::Window, path: PathBuf, name: &str) {
    let mut text = tr("MAN flagged this Windows program (.exe) as malware, which can harm your data or privacy. It should not be run and should be moved to the trash.").to_string();
    if !name.is_empty() {
        text.push_str("\n\n");
        text.push_str(&trf("Detected as: {name}", &[("name", &name)]));
    }
    let trash_path = path.clone();
    let buttons: Vec<Btn> = vec![
        (
            tr("_Move to Trash"),
            "",
            Box::new(move |w, _| match gio::File::for_path(&trash_path).trash(gio::Cancellable::NONE) {
                Ok(()) => w.close(),
                Err(e) => show_error(w, tr("MAN Flagged This Program as Malware"), &e.to_string()),
            }),
        ),
        cancel_button(tr("_Cancel")),
        (
            tr("_Bypass and Run"),
            "danger",
            Box::new(move |w, b| {
                let (w, p, b) = (w.clone(), path.clone(), b.clone());
                b.set_sensitive(false);
                glib::spawn_future_local(async move {
                    let allowed = gio::spawn_blocking(|| Command::new("pkexec").arg(BYPASS_HELPER).stdin(Stdio::null()).status().map(|s| s.success()).unwrap_or(false)).await.unwrap_or(false);
                    b.set_sensitive(true);
                    if allowed {
                        confirm_bypass(&w, p);
                    }
                });
            }),
        ),
    ];
    show(window, "man-alert-danger", tr("MAN Flagged This Program as Malware"), &text, buttons, false);
}

fn open_file(app: &adw::Application, path: PathBuf) {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let window: gtk::Window = adw::Window::builder().application(app).resizable(false).icon_name(APP_ID).title(tr("Windows Program")).build().upcast();
    window.set_widget_name(&format!("file:{name}"));
    window.add_css_class("man-alert");
    let keys = gtk::EventControllerKey::new();
    {
        let w = window.clone();
        keys.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Escape {
                w.close();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
    }
    window.add_controller(keys);

    let msi = is_msi(&path);
    let format = if msi { Format::Runs } else { format_of(pe_machine(&path)) };
    match format {
        Format::NotWindows => {
            show_error(&window, tr("This Is Not a Windows Program"), tr("This file cannot be run."));
            window.present();
        }
        Format::OtherProcessor(arch) => {
            show_error(&window, tr("This Program Cannot Run on This Computer"), &trf("It was made for {arch} processors.", &[("arch", &arch)]));
            window.present();
        }
        Format::Runs => {
            // Usually known since the download: the popup then opens with the
            // answer. Only a real wait shows the waiting state.
            show(&window, "man-alert-warning", tr("Checking This Program for Malware"), "", vec![cancel_button(tr("_Cancel"))], true);
            let w = window.clone();
            let shown = glib::timeout_add_local_once(std::time::Duration::from_millis(400), move || w.present());
            let window = window.clone();
            glib::spawn_future_local(async move {
                let p = path.clone();
                let result = gio::spawn_blocking(move || malscan::check(&p)).await.unwrap_or(Scan::Failed);
                shown.remove();
                match result {
                    Scan::Clean => warn_page(&window, path, true),
                    Scan::Failed => warn_page(&window, path, false),
                    Scan::Flagged(name) => flagged_page(&window, path, &name),
                }
                window.present();
            });
        }
    }
}

pub(crate) fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(include_str!("../../../data/gtk/man-alerts.css"));
    let own = gtk::CssProvider::new();
    own.load_from_string(CSS);
    if let Some(display) = gtk::gdk::Display::default() {
        #[allow(deprecated)]
        {
            gtk::style_context_add_provider_for_display(&display, &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
            gtk::style_context_add_provider_for_display(&display, &own, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        }
    }
}

pub fn main() -> glib::ExitCode {
    crate::i18n::init();
    let app = adw::Application::builder().application_id(APP_ID).flags(gio::ApplicationFlags::HANDLES_OPEN | gio::ApplicationFlags::NON_UNIQUE).build();
    app.connect_startup(|_| load_css());
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
