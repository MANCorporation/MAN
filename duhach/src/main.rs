//! DUHACH! — installs MAN from Windows without a USB stick.
//!
//! It checks the PC, verifies a MAN ISO, shrinks Windows (C:), puts the MAN
//! installer on a small FAT partition at the end of the freed space, adds a
//! one-time firmware boot entry for it and restarts. The MAN installer then
//! offers "Install Alongside Windows" for the free space.
#![windows_subsystem = "windows"]
mod sys;

use adw::prelude::*;
use gtk::{gio, glib};
use std::cell::RefCell;
use std::rc::Rc;
use sys::{human, Plan, Progress, SysInfo, GIB, MIN_MAN};

const APP_ID: &str = "com.mancorporation.Duhach";

const CSS: &str = r#"
.hero-icon { -gtk-icon-size: 128px; }
.hero-title { font-weight: 900; font-size: 2.4em; letter-spacing: 0.02em; }
.step-title { font-weight: 800; font-size: 1.6em; }
.big-progress trough, .big-progress progress { min-height: 10px; border-radius: 99px; }
.space-legend { font-size: 0.9em; }
.legend-dot { min-width: 12px; min-height: 12px; border-radius: 99px; }
.legend-dot.windows { background: #77767b; }
.legend-dot.free { background: alpha(#77767b, 0.35); }
.legend-dot.man { background: #1c71d8; }
.legend-dot.installer { background: #2ec27e; }
.big-number { font-weight: 800; font-size: 1.5em; font-feature-settings: "tnum"; }
.step-row.pending { opacity: 0.55; }
.done-page image.icon { color: var(--success-color); }
.error-icon { color: var(--error-color); }
"#;

struct State {
    sys: Option<SysInfo>,
    iso: Option<sys::IsoInfo>,
    man_bytes: u64,
    backed_up: bool,
}

struct Ctx {
    window: adw::ApplicationWindow,
    nav: adw::NavigationView,
    toasts: adw::ToastOverlay,
    state: RefCell<State>,
    busy: std::cell::Cell<bool>,
}
type C = Rc<Ctx>;

impl Ctx {
    fn toast(&self, text: &str) {
        self.toasts.add_toast(adw::Toast::new(text));
    }
    fn sys(&self) -> SysInfo {
        self.state.borrow().sys.clone().unwrap_or_default()
    }
    fn installer_bytes(&self) -> u64 {
        self.state.borrow().iso.as_ref().map(|i| i.partition_bytes()).unwrap_or(2 * GIB)
    }
}

fn main() -> glib::ExitCode {
    // Next to DUHACH!.exe: share/ with icons and settings schemas.
    #[cfg(windows)]
    if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
        std::env::set_var("XDG_DATA_DIRS", dir.join("share"));
        std::env::set_var("GSETTINGS_SCHEMA_DIR", dir.join("share").join("glib-2.0").join("schemas"));
    }
    let app = adw::Application::builder().application_id(APP_ID).flags(gio::ApplicationFlags::NON_UNIQUE).build();
    app.connect_startup(|_| {
        let provider = gtk::CssProvider::new();
        provider.load_from_string(CSS);
        if let Some(display) = gtk::gdk::Display::default() {
            #[allow(deprecated)]
            gtk::style_context_add_provider_for_display(&display, &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
            let theme = gtk::IconTheme::for_display(&display);
            for dir in icon_dirs() {
                theme.add_search_path(dir);
            }
        }
    });
    app.connect_activate(build);
    app.run()
}

/// Where DUHACH!'s own icon lives: next to the program, or the source tree.
fn icon_dirs() -> Vec<std::path::PathBuf> {
    let mut v = vec![];
    if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
        v.push(dir.join("share").join("icons"));
        v.push(dir.join("..").join("..").join("data").join("icons"));
    }
    v.push(std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/data/icons")));
    v
}

fn build(app: &adw::Application) {
    let window = adw::ApplicationWindow::builder().application(app).title("DUHACH!").default_width(820).default_height(680).icon_name(APP_ID).build();
    gtk::Window::set_default_icon_name(APP_ID);
    let nav = adw::NavigationView::new();
    let toasts = adw::ToastOverlay::new();
    toasts.set_child(Some(&nav));
    window.set_content(Some(&toasts));
    let ctx: C = Rc::new(Ctx {
        window: window.clone(),
        nav,
        toasts,
        state: RefCell::new(State { sys: None, iso: None, man_bytes: 0, backed_up: false }),
        busy: Default::default(),
    });
    {
        let ctx = ctx.clone();
        window.connect_close_request(move |_| {
            if ctx.busy.get() {
                ctx.toast("DUHACH! is changing your disk. Please wait until it finishes.");
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
    }
    ctx.nav.replace(&[welcome_page(&ctx)]);
    window.present();
}

// ───────────────────────────── Helpers ─────────────────────────────

fn pill(label: &str, class: Option<&str>) -> gtk::Button {
    let b = gtk::Button::builder().label(label).use_underline(true).halign(gtk::Align::Center).build();
    b.add_css_class("pill");
    if let Some(c) = class {
        b.add_css_class(c);
    }
    b
}

fn label(text: &str, classes: &[&str]) -> gtk::Label {
    let l = gtk::Label::builder().label(text).wrap(true).justify(gtk::Justification::Center).build();
    for c in classes {
        l.add_css_class(c);
    }
    l
}

fn icon(name: &str) -> gtk::Image {
    gtk::Image::from_icon_name(name)
}

fn page(title: &str, tag: &str, content: &impl IsA<gtk::Widget>, bottom: Option<&gtk::Widget>, can_go_back: bool) -> adw::NavigationPage {
    let tv = adw::ToolbarView::new();
    tv.add_top_bar(&adw::HeaderBar::new());
    tv.set_content(Some(content));
    if let Some(b) = bottom {
        let bar = gtk::Box::builder().orientation(gtk::Orientation::Vertical).margin_top(12).margin_bottom(18).margin_start(12).margin_end(12).spacing(8).build();
        bar.append(b);
        tv.add_bottom_bar(&bar);
    }
    adw::NavigationPage::builder().title(title).tag(tag).child(&tv).can_pop(can_go_back).build()
}

fn boxed_list() -> gtk::ListBox {
    let l = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).build();
    l.add_css_class("boxed-list");
    l
}

fn info_row(title: &str, value: &str, icon_name: &str) -> adw::ActionRow {
    let row = adw::ActionRow::builder().title(title).subtitle(glib::markup_escape_text(value)).build();
    row.add_css_class("property");
    row.add_prefix(&icon(icon_name));
    row
}

fn scrolled(child: &impl IsA<gtk::Widget>) -> gtk::ScrolledWindow {
    let clamp = adw::Clamp::builder().maximum_size(600).child(child).margin_top(12).margin_bottom(24).margin_start(12).margin_end(12).build();
    gtk::ScrolledWindow::builder().hscrollbar_policy(gtk::PolicyType::Never).vexpand(true).child(&clamp).build()
}

fn vbox(spacing: i32) -> gtk::Box {
    gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(spacing).build()
}

// ───────────────────────────── Welcome ─────────────────────────────

fn welcome_page(ctx: &C) -> adw::NavigationPage {
    let content = vbox(18);
    content.set_valign(gtk::Align::Center);
    let hero = icon(APP_ID);
    hero.add_css_class("hero-icon");
    content.append(&hero);
    content.append(&label("DUHACH!", &["hero-title"]));
    content.append(&label(
        "Install MAN on this PC, right from Windows. No USB stick needed.\nWindows stays, and you choose which one to start.",
        &["dim-label"],
    ));

    let group = adw::PreferencesGroup::builder().title("Checking This PC").build();
    let list = boxed_list();
    let spinner_row = adw::ActionRow::builder().title("Looking at your PC…").build();
    spinner_row.add_prefix(&adw::Spinner::new());
    list.append(&spinner_row);
    group.add(&list);
    content.append(&group);

    let next = pill("_Get Started", Some("suggested-action"));
    next.set_sensitive(false);
    let firmware = gtk::Button::builder().label("Restart to _UEFI Settings…").use_underline(true).halign(gtk::Align::Center).visible(false).build();
    firmware.add_css_class("pill");
    let ahci = gtk::Button::builder().label("Switch to _AHCI…").use_underline(true).halign(gtk::Align::Center).visible(false).build();
    ahci.add_css_class("pill");
    ahci.add_css_class("suggested-action");
    let bottom = vbox(10);
    bottom.append(&next);
    bottom.append(&ahci);
    bottom.append(&firmware);
    if sys::demo() {
        let l = label("Demo mode: this is not Windows, so nothing on this computer is changed.", &["dim-label", "caption"]);
        bottom.append(&l);
    }

    {
        let ctx = ctx.clone();
        let (list, next, firmware, ahci) = (list.clone(), next.clone(), firmware.clone(), ahci.clone());
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(sys::system_info).await.unwrap_or_else(|_| Err("The check crashed.".into()));
            list.remove_all();
            match result {
                Err(e) => {
                    let row = adw::ActionRow::builder().title("This PC could not be checked").subtitle(glib::markup_escape_text(&e)).build();
                    let img = icon("dialog-error-symbolic");
                    img.add_css_class("error-icon");
                    row.add_prefix(&img);
                    list.append(&row);
                }
                Ok(info) => {
                    let checks = sys::checks(&info, 2 * GIB);
                    let blocking = checks.iter().any(|c| c.blocking);
                    for c in &checks {
                        let row = adw::ActionRow::builder().title(c.title.as_str()).subtitle(glib::markup_escape_text(&c.detail)).build();
                        let img = icon(if c.ok {
                            "object-select-symbolic"
                        } else if c.blocking {
                            "dialog-error-symbolic"
                        } else {
                            "dialog-warning-symbolic"
                        });
                        img.add_css_class(if c.ok { "success" } else if c.blocking { "error" } else { "warning" });
                        row.add_prefix(&img);
                        list.append(&row);
                    }
                    firmware.set_visible((info.secure_boot || info.ahci_pending) && info.uefi);
                    ahci.set_visible(info.raid_mode && !info.ahci_pending && info.admin && info.uefi);
                    if info.raid_mode {
                        next.set_visible(false);
                    }
                    let prepared = info.prepared.is_some();
                    ctx.state.borrow_mut().sys = Some(info);
                    if prepared {
                        ctx.nav.replace(&[prepared_page(&ctx)]);
                        return;
                    }
                    next.set_sensitive(!blocking);
                    if !blocking {
                        next.grab_focus();
                    }
                }
            }
        });
    }
    {
        let ctx = ctx.clone();
        next.connect_clicked(move |_| ctx.nav.push(&iso_page(&ctx)));
    }
    {
        let ctx = ctx.clone();
        firmware.connect_clicked(move |_| secure_boot_dialog(&ctx));
    }
    {
        let ctx = ctx.clone();
        ahci.connect_clicked(move |_| ahci_dialog(&ctx));
    }
    page("DUHACH!", "welcome", &scrolled(&content), Some(bottom.upcast_ref()), false)
}

fn ahci_dialog(ctx: &C) {
    let s = ctx.sys();
    let dialog = adw::AlertDialog::new(
        Some("Switch the Storage Mode to AHCI"),
        Some(&format!(
            "MAN cannot read disks in RAID / Intel RST mode. Changing it only in the UEFI settings would stop Windows from starting, so DUHACH! prepares Windows first:\n\n\
             1. Your PC restarts into its UEFI settings (if it starts Windows instead, restart and press F2 or Del while the logo shows).\n\
             2. Find the storage mode (often Advanced → SATA Configuration, “SATA Mode”, “Storage Controller” or “VMD”) and set it from RAID / Intel RST to AHCI. Save and exit.\n\
             3. Windows starts once in Safe Mode, then restarts normally by itself.\n\
             4. Open DUHACH! again.\n\n\
             Windows and your files are not changed.{}",
            if s.bitlocker { " BitLocker is paused for the next restarts; keep your recovery key at hand." } else { "" }
        )),
    );
    dialog.add_response("cancel", "_Cancel");
    dialog.add_response("go", "_Prepare and Restart");
    dialog.set_response_appearance("go", adw::ResponseAppearance::Suggested);
    dialog.set_close_response("cancel");
    let ctx = ctx.clone();
    glib::spawn_future_local(async move {
        if dialog.choose_future(Some(&ctx.window)).await == "go" {
            let bl = s.bitlocker;
            match gio::spawn_blocking(move || sys::switch_to_ahci(bl)).await.unwrap_or_else(|_| Err("The change crashed.".into())) {
                Ok(()) => {}
                Err(e) => ctx.toast(&e),
            }
        }
    });
}

fn secure_boot_dialog(ctx: &C) {
    let s = ctx.sys();
    let dialog = adw::AlertDialog::new(
        Some(if s.ahci_pending { "Set the Storage Mode to AHCI" } else { "Turn Off Secure Boot" }),
        Some(if s.ahci_pending {
            "Your PC restarts into its UEFI settings (if not, press F2 or Del while the logo shows). Set the storage mode from RAID / Intel RST to AHCI, save and exit. Windows starts once in Safe Mode and then normally; then open DUHACH! again."
        } else {
            "Your PC restarts into its UEFI settings (if not, press F2 or Del while the logo shows). Look for “Secure Boot” (often under Security or Boot), set it to Disabled, save and exit. Windows starts again; then open DUHACH! once more.\n\nWindows keeps working with Secure Boot off."
        }),
    );
    dialog.add_response("cancel", "_Cancel");
    dialog.add_response("restart", "_Restart Now");
    dialog.set_response_appearance("restart", adw::ResponseAppearance::Suggested);
    dialog.set_close_response("cancel");
    let ctx = ctx.clone();
    glib::spawn_future_local(async move {
        if dialog.choose_future(Some(&ctx.window)).await == "restart" {
            if let Err(e) = sys::restart(true) {
                ctx.toast(&e);
            }
        }
    });
}

// ──────────────────────────── Choose ISO ────────────────────────────

fn iso_page(ctx: &C) -> adw::NavigationPage {
    let content = vbox(18);
    let hero = icon("media-optical-symbolic");
    hero.set_pixel_size(96);
    hero.add_css_class("dim-label");
    content.append(&hero);
    content.append(&label("Choose MAN", &["step-title"]));
    content.append(&label("Select the MAN ISO file you downloaded. DUHACH! checks that it is a complete MAN system.", &["dim-label"]));

    let list = boxed_list();
    let row = adw::ActionRow::builder().title("MAN ISO File").subtitle("No file chosen").activatable(true).build();
    row.add_prefix(&icon("folder-open-symbolic"));
    let choose = gtk::Button::builder().label("_Choose…").use_underline(true).valign(gtk::Align::Center).build();
    row.add_suffix(&choose);
    row.set_activatable_widget(Some(&choose));
    list.append(&row);
    content.append(&list);

    let next = pill("_Next", Some("suggested-action"));
    next.set_sensitive(ctx.state.borrow().iso.is_some());
    if let Some(iso) = &ctx.state.borrow().iso {
        show_iso(&row, iso);
    }
    {
        let ctx = ctx.clone();
        let (row, next) = (row.clone(), next.clone());
        choose.connect_clicked(move |btn| {
            let filter = gtk::FileFilter::new();
            filter.set_name(Some("MAN ISO files"));
            filter.add_pattern("*.iso");
            filter.add_pattern("*.ISO");
            let filters = gio::ListStore::new::<gtk::FileFilter>();
            filters.append(&filter);
            let dialog = gtk::FileDialog::builder().title("Choose the MAN ISO").filters(&filters).modal(true).build();
            let ctx = ctx.clone();
            let (row, next, btn) = (row.clone(), next.clone(), btn.clone());
            glib::spawn_future_local(async move {
                let Ok(file) = dialog.open_future(Some(&ctx.window)).await else { return };
                let Some(path) = file.path() else { return ctx.toast("Choose a file on this PC.") };
                let path = path.to_string_lossy().into_owned();
                row.set_subtitle("Checking…");
                let spinner = adw::Spinner::new();
                row.add_suffix(&spinner);
                btn.set_sensitive(false);
                next.set_sensitive(false);
                let p2 = path.clone();
                let r = gio::spawn_blocking(move || sys::inspect_iso(&p2)).await.unwrap_or_else(|_| Err("The check crashed.".into()));
                row.remove(&spinner);
                btn.set_sensitive(true);
                match r {
                    Ok(iso) => {
                        show_iso(&row, &iso);
                        ctx.state.borrow_mut().iso = Some(iso);
                        next.set_sensitive(true);
                        next.grab_focus();
                    }
                    Err(e) => {
                        ctx.state.borrow_mut().iso = None;
                        row.set_title("MAN ISO File");
                        row.set_subtitle(&glib::markup_escape_text(&format!("{}\n{e}", file_name(&path))));
                    }
                }
            });
        });
    }
    {
        let ctx = ctx.clone();
        next.connect_clicked(move |_| ctx.nav.push(&space_page(&ctx)));
    }
    page("Choose MAN", "iso", &scrolled(&content), Some(next.upcast_ref()), true)
}

fn file_name(path: &str) -> String {
    std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.into())
}

fn show_iso(row: &adw::ActionRow, iso: &sys::IsoInfo) {
    row.set_title(&glib::markup_escape_text(&format!("MAN {}", iso.version())));
    row.set_subtitle(&glib::markup_escape_text(&format!("{}  ·  {}  ·  verified", file_name(&iso.path), human(iso.bytes))));
}

// ──────────────────────────── Space ────────────────────────────────

/// The disk as a bar: Windows (used, free) and the space for MAN.
fn space_bar(ctx: &C, man: Rc<std::cell::Cell<u64>>) -> gtk::DrawingArea {
    let s = ctx.sys();
    let installer = ctx.installer_bytes();
    let area = gtk::DrawingArea::builder().content_height(44).hexpand(true).build();
    area.set_draw_func(move |_, cr, w, h| {
        let (w, h) = (w as f64, h as f64);
        let total = s.c_size as f64;
        let used = (s.c_size - s.c_free) as f64;
        let man_b = man.get() as f64;
        let win = total - man_b - installer as f64;
        let segs = [
            (used / total, (0.467, 0.463, 0.482, 1.0)),
            ((win - used).max(0.0) / total, (0.467, 0.463, 0.482, 0.35)),
            (man_b / total, (0.11, 0.443, 0.847, 1.0)),
            (installer as f64 / total, (0.18, 0.76, 0.494, 1.0)),
        ];
        let r = 10.0;
        // Rounded clip for the whole bar.
        cr.new_sub_path();
        cr.arc(w - r, r, r, -std::f64::consts::FRAC_PI_2, 0.0);
        cr.arc(w - r, h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
        cr.arc(r, h - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
        cr.arc(r, r, r, std::f64::consts::PI, 1.5 * std::f64::consts::PI);
        cr.close_path();
        cr.clip();
        let mut x = 0.0;
        for (frac, (cr_, cg, cb, ca)) in segs {
            let sw = (frac * w).max(if frac > 0.0 { 2.0 } else { 0.0 });
            cr.set_source_rgba(cr_, cg, cb, ca);
            cr.rectangle(x, 0.0, sw + 0.5, h);
            let _ = cr.fill();
            x += sw;
        }
    });
    area
}

fn legend(class: &str, title: &str) -> (gtk::Box, gtk::Label) {
    let b = gtk::Box::builder().spacing(8).build();
    let dot = gtk::Box::builder().valign(gtk::Align::Center).build();
    dot.add_css_class("legend-dot");
    dot.add_css_class(class);
    b.append(&dot);
    let text = vbox(0);
    let t = gtk::Label::builder().label(title).xalign(0.0).build();
    t.add_css_class("space-legend");
    t.add_css_class("dim-label");
    let v = gtk::Label::builder().xalign(0.0).build();
    v.add_css_class("heading");
    text.append(&t);
    text.append(&v);
    b.append(&text);
    (b, v)
}

fn space_page(ctx: &C) -> adw::NavigationPage {
    let s = ctx.sys();
    let installer = ctx.installer_bytes();
    let max = s.max_man(installer);
    let default = {
        let cur = ctx.state.borrow().man_bytes;
        if cur >= MIN_MAN && cur <= max {
            cur
        } else {
            // A third of C:, at least 40 GB, never more than possible.
            (s.c_size / 3).clamp(MIN_MAN.max(40 * GIB).min(max), max) / GIB * GIB
        }
    };
    ctx.state.borrow_mut().man_bytes = default;
    let man = Rc::new(std::cell::Cell::new(default));

    let content = vbox(18);
    content.append(&label("Make Room for MAN", &["step-title"]));
    content.append(&label(
        &format!("Windows (C:) gives part of its free space to MAN. Your files and programs stay where they are. {} on {}.", human(s.disk_size), s.disk_model),
        &["dim-label"],
    ));

    let card = vbox(14);
    card.add_css_class("card");
    card.set_margin_top(6);
    let inner = vbox(14);
    inner.set_margin_top(18);
    inner.set_margin_bottom(18);
    inner.set_margin_start(18);
    inner.set_margin_end(18);
    let bar = space_bar(ctx, man.clone());
    inner.append(&bar);
    let legends = gtk::Box::builder().spacing(18).homogeneous(true).build();
    let (l1, v_used) = legend("windows", "Windows files");
    let (l2, v_free) = legend("free", "Windows keeps free");
    let (l3, v_man) = legend("man", "MAN");
    let (l4, v_inst) = legend("installer", "MAN installer");
    for l in [&l1, &l2, &l3, &l4] {
        legends.append(l);
    }
    v_used.set_label(&human(s.c_size - s.c_free));
    v_inst.set_label(&human(installer));
    inner.append(&legends);
    card.append(&inner);
    content.append(&card);

    let group = adw::PreferencesGroup::new();
    let gb_max = (max / GIB) as f64;
    let gb_min = (MIN_MAN / GIB) as f64;
    let adj = gtk::Adjustment::new((default / GIB) as f64, gb_min, gb_max.max(gb_min), 1.0, 10.0, 0.0);
    let spin = adw::SpinRow::builder().title("Space for MAN").subtitle(format!("Between {} and {} GB", gb_min, gb_max)).adjustment(&adj).digits(0).build();
    spin.add_prefix(&icon("drive-harddisk-symbolic"));
    group.add(&spin);
    let scale = gtk::Scale::builder().adjustment(&adj).draw_value(false).hexpand(true).build();
    scale.add_mark(gb_min, gtk::PositionType::Bottom, Some(&format!("{} GB", gb_min)));
    scale.add_mark(gb_max, gtk::PositionType::Bottom, Some(&format!("{} GB", gb_max)));
    content.append(&scale);
    content.append(&group);
    let note = label(
        &format!("The MAN installer partition ({}) is temporary: MAN takes it over after installing.", human(installer)),
        &["dim-label", "caption"],
    );
    content.append(&note);

    let update = {
        let (ctx, man, bar, v_free, v_man) = (ctx.clone(), man.clone(), bar.clone(), v_free.clone(), v_man.clone());
        let s = s.clone();
        move |gb: f64| {
            let bytes = (gb.round() as u64) * GIB;
            man.set(bytes);
            ctx.state.borrow_mut().man_bytes = bytes;
            let win_free = s.c_free.saturating_sub(bytes + installer);
            v_free.set_label(&human(win_free));
            v_man.set_label(&human(bytes));
            bar.queue_draw();
        }
    };
    update(adj.value());
    adj.connect_value_changed(move |a| update(a.value()));

    let next = pill("_Next", Some("suggested-action"));
    {
        let ctx = ctx.clone();
        next.connect_clicked(move |_| ctx.nav.push(&ready_page(&ctx)));
    }
    page("Make Room", "space", &scrolled(&content), Some(next.upcast_ref()), true)
}

// ───────────────────────────── Ready ───────────────────────────────

fn plan(ctx: &C) -> Option<Plan> {
    let st = ctx.state.borrow();
    Some(Plan { iso: st.iso.clone()?, man_bytes: st.man_bytes, installer_bytes: st.iso.as_ref()?.partition_bytes(), sys: st.sys.clone()? })
}

fn ready_page(ctx: &C) -> adw::NavigationPage {
    let plan = plan(ctx).expect("plan");
    let s = &plan.sys;
    let content = vbox(18);
    content.append(&label("Ready to Prepare MAN", &["step-title"]));

    let what = adw::PreferencesGroup::builder().title("What DUHACH! Will Do").build();
    what.add(&info_row("Shrink Windows (C:)", &format!("{} → {}", human(s.c_size), human(plan.new_c_size())), "drive-harddisk-symbolic"));
    what.add(&info_row("Space for MAN", &human(plan.man_bytes), "computer-symbolic"));
    what.add(&info_row("MAN Installer Partition", &format!("{}, with MAN {} copied onto it", human(plan.installer_bytes), plan.iso.version()), "media-flash-symbolic"));
    what.add(&info_row("Boot Menu", "The next restart starts the MAN installer, once. Windows stays the default.", "system-reboot-symbolic"));
    if s.bitlocker {
        what.add(&info_row("BitLocker", "Paused for the next three restarts. Keep your recovery key at hand.", "channel-secure-symbolic"));
    }
    content.append(&what);

    let next_steps = adw::PreferencesGroup::builder().title("After the Restart").build();
    next_steps.add(&info_row(
        "In the MAN Installer",
        "Choose “Install Alongside Windows”. MAN goes into the space made here and adds Windows to its start menu.",
        "go-next-symbolic",
    ));
    content.append(&next_steps);

    let safety = adw::PreferencesGroup::new();
    let backed = adw::SwitchRow::builder()
        .title("I have a backup of my important files")
        .subtitle("Changing partitions is safe, but a power cut in the middle could lose data.")
        .build();
    backed.set_active(ctx.state.borrow().backed_up);
    safety.add(&backed);
    content.append(&safety);

    let go = pill("_Prepare MAN", Some("destructive-action"));
    go.set_sensitive(backed.is_active());
    {
        let (ctx, go) = (ctx.clone(), go.clone());
        backed.connect_active_notify(move |r| {
            ctx.state.borrow_mut().backed_up = r.is_active();
            go.set_sensitive(r.is_active());
        });
    }
    {
        let ctx = ctx.clone();
        go.connect_clicked(move |_| confirm(&ctx));
    }
    page("Ready", "ready", &scrolled(&content), Some(go.upcast_ref()), true)
}

fn confirm(ctx: &C) {
    let Some(plan) = plan(ctx) else { return };
    let dialog = adw::AlertDialog::new(
        Some("Shrink Windows and Prepare MAN?"),
        Some(&format!(
            "Drive C: shrinks from {} to {} and a {} MAN installer partition is created. Close your other programs first. This takes a few minutes.",
            human(plan.sys.c_size),
            human(plan.new_c_size()),
            human(plan.installer_bytes)
        )),
    );
    dialog.add_response("cancel", "_Cancel");
    dialog.add_response("go", "_Prepare MAN");
    dialog.set_response_appearance("go", adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    let ctx = ctx.clone();
    glib::spawn_future_local(async move {
        if dialog.choose_future(Some(&ctx.window)).await == "go" {
            run_prepare(&ctx, plan);
        }
    });
}

// ──────────────────────────── Progress ─────────────────────────────

fn run_prepare(ctx: &C, plan: Plan) {
    let content = vbox(18);
    content.set_valign(gtk::Align::Center);
    let hero = icon(APP_ID);
    hero.add_css_class("hero-icon");
    content.append(&hero);
    content.append(&label("Preparing MAN", &["step-title"]));
    content.append(&label("Please keep your PC on and plugged in.", &["dim-label"]));
    let list = boxed_list();
    let mut rows = vec![];
    for title in sys::STEPS {
        let row = adw::ActionRow::builder().title(title).build();
        row.add_css_class("step-row");
        row.add_css_class("pending");
        let stack = gtk::Stack::new();
        stack.add_named(&icon("content-loading-symbolic"), Some("pending"));
        stack.add_named(&adw::Spinner::new(), Some("running"));
        let done = icon("object-select-symbolic");
        done.add_css_class("success");
        stack.add_named(&done, Some("done"));
        let failed = icon("dialog-error-symbolic");
        failed.add_css_class("error");
        stack.add_named(&failed, Some("failed"));
        row.add_prefix(&stack);
        list.append(&row);
        rows.push((row, stack));
    }
    content.append(&list);
    let bar = gtk::ProgressBar::builder().show_text(false).build();
    bar.add_css_class("big-progress");
    let detail = label("", &["dim-label", "caption"]);
    content.append(&bar);
    content.append(&detail);
    let p = page("Preparing", "progress", &scrolled(&content), None, false);
    ctx.nav.push(&p);
    ctx.busy.set(true);

    let (tx, rx) = async_channel::unbounded::<Result<Progress, Result<(), String>>>();
    {
        let tx = tx.clone();
        std::thread::spawn(move || {
            let r = sys::prepare(&plan, &|p| {
                let _ = tx.send_blocking(Ok(p));
            });
            let _ = tx.send_blocking(Err(r));
        });
    }
    let ctx = ctx.clone();
    glib::spawn_future_local(async move {
        let mut current = 0usize;
        while let Ok(msg) = rx.recv().await {
            match msg {
                Ok(Progress::Step(i)) => {
                    for (j, (row, stack)) in rows.iter().enumerate() {
                        if j < i {
                            stack.set_visible_child_name("done");
                            row.remove_css_class("pending");
                        } else if j == i {
                            stack.set_visible_child_name("running");
                            row.remove_css_class("pending");
                        }
                    }
                    current = i;
                    detail.set_label("");
                    bar.set_fraction(i as f64 / sys::STEPS.len() as f64);
                }
                Ok(Progress::Detail(f, text)) => {
                    // Copying is the long step; it fills most of the bar.
                    let weights = [0.15, 0.05, 0.65, 0.1, 0.05];
                    let base: f64 = weights[..current].iter().sum();
                    bar.set_fraction(base + weights[current] * f.clamp(0.0, 1.0));
                    detail.set_label(&text);
                }
                Err(result) => {
                    ctx.busy.set(false);
                    match result {
                        Ok(()) => {
                            for (_, stack) in &rows {
                                stack.set_visible_child_name("done");
                            }
                            bar.set_fraction(1.0);
                            ctx.nav.replace(&[done_page(&ctx)]);
                        }
                        Err(e) => {
                            rows[current].1.set_visible_child_name("failed");
                            ctx.nav.push(&error_page(&ctx, &e));
                        }
                    }
                    break;
                }
            }
        }
    });
}

fn done_page(ctx: &C) -> adw::NavigationPage {
    let status = adw::StatusPage::builder()
        .icon_name("object-select-symbolic")
        .title("MAN Is Ready to Install")
        .description(
            "Restart your PC: it starts the MAN installer once.\nThere, choose <b>Install Alongside Windows</b>.\n\nWindows is not changed and stays in the start menu, so you can always go back.",
        )
        .build();
    status.add_css_class("done-page");
    let restart = pill("_Restart Now", Some("suggested-action"));
    let later = pill("_Later", None);
    let buttons = vbox(10);
    buttons.append(&restart);
    buttons.append(&later);
    status.set_child(Some(&buttons));
    {
        let ctx = ctx.clone();
        restart.connect_clicked(move |_| {
            if let Err(e) = sys::restart(false) {
                ctx.toast(&e);
            }
        });
    }
    {
        let ctx = ctx.clone();
        later.connect_clicked(move |_| ctx.window.close());
    }
    page("Done", "done", &status, None, false)
}

fn error_page(ctx: &C, message: &str) -> adw::NavigationPage {
    let status = adw::StatusPage::builder()
        .icon_name("dialog-error-symbolic")
        .title("MAN Could Not Be Prepared")
        .description(glib::markup_escape_text(&format!(
            "{message}\n\nDUHACH! undid its changes: drive C: has its old size again and no boot entry was left behind."
        )))
        .build();
    let close = pill("_Close", None);
    {
        let ctx = ctx.clone();
        close.connect_clicked(move |_| ctx.window.close());
    }
    status.set_child(Some(&close));
    page("Error", "error", &status, None, false)
}

// ───────────────────────── Already prepared ────────────────────────

fn prepared_page(ctx: &C) -> adw::NavigationPage {
    let s = ctx.sys();
    let prep = s.prepared.clone().unwrap_or_default();
    let version = if prep.version.is_empty() { String::new() } else { format!(" {}", prep.version) };
    let status = adw::StatusPage::builder()
        .icon_name(APP_ID)
        .title(format!("The MAN{version} Installer Is Ready"))
        .description(if s.man_installed {
            "MAN is installed on this PC. The installer partition is no longer needed and can be removed.".to_string()
        } else {
            format!(
                "DUHACH! already prepared this PC ({} installer partition). Restart to install MAN, or remove the installer and give the space back to Windows.",
                human(prep.size)
            )
        })
        .build();
    let buttons = vbox(10);
    let restart = pill("_Restart into the MAN Installer", Some("suggested-action"));
    restart.set_visible(!s.man_installed);
    let remove = pill(if s.man_installed { "_Remove the Installer Partition" } else { "Re_move and Give Space Back" }, Some("destructive-action"));
    buttons.append(&restart);
    buttons.append(&remove);
    status.set_child(Some(&buttons));
    {
        let ctx = ctx.clone();
        restart.connect_clicked(move |_| {
            if let Err(e) = sys::restart(false) {
                ctx.toast(&e);
            }
        });
    }
    {
        let ctx = ctx.clone();
        remove.connect_clicked(move |btn| {
            let s = ctx.sys();
            let dialog = adw::AlertDialog::new(
                Some("Remove the MAN Installer?"),
                Some(if s.man_installed {
                    "The installer partition and its boot entry are deleted. The installed MAN is not changed."
                } else {
                    "The installer partition and its boot entry are deleted and drive C: gets its space back."
                }),
            );
            dialog.add_response("cancel", "_Cancel");
            dialog.add_response("remove", "_Remove");
            dialog.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
            dialog.set_close_response("cancel");
            let ctx = ctx.clone();
            let btn = btn.clone();
            glib::spawn_future_local(async move {
                if dialog.choose_future(Some(&ctx.window)).await != "remove" {
                    return;
                }
                btn.set_sensitive(false);
                ctx.busy.set(true);
                let r = gio::spawn_blocking(move || sys::remove(&s)).await.unwrap_or_else(|_| Err("The removal crashed.".into()));
                ctx.busy.set(false);
                match r {
                    Ok(()) => {
                        ctx.state.borrow_mut().sys = None;
                        ctx.nav.replace(&[welcome_page(&ctx)]);
                        ctx.toast("The MAN installer was removed");
                    }
                    Err(e) => {
                        btn.set_sensitive(true);
                        ctx.toast(&e);
                    }
                }
            });
        });
    }
    page("DUHACH!", "prepared", &status, None, false)
}
