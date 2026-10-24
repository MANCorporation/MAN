//! "Install MAN": the live-session installer.
use crate::i18n::{tr, trf, tr_dyn};
use crate::backend;
use crate::ui::*;
use adw::prelude::*;
use gtk::{gio, glib};
use man_common::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Weight of each backend step in the overall progress bar.
const WEIGHTS: [f64; 7] = [1.0, 4.0, 62.0, 16.0, 10.0, 3.0, 4.0];

#[derive(Default)]
struct State {
    probe: Option<Probe>,
    mode: Mode,
    disk: Option<String>,
    reinstall: Option<String>,
    reinstall_pass: String,
    alongside: Option<String>,
    esp: Option<String>,
    format_esp: bool,
    root: Option<String>,
    boot: Option<String>,
    encrypt: bool,
    passphrase: String,
    passphrase2: String,
    timezone: String,
    keyboard: String,
}

struct Ctx {
    window: adw::ApplicationWindow,
    nav: adw::NavigationView,
    toasts: adw::ToastOverlay,
    state: RefCell<State>,
    busy: Cell<bool>,
    timezones: Rc<Vec<(String, String, String)>>,
    keyboards: Rc<Vec<(String, String, String)>>,
}

type C = Rc<Ctx>;

impl Ctx {
    fn toast(&self, text: &str) {
        self.toasts.add_toast(adw::Toast::new(text));
    }

    fn probe(&self) -> Probe {
        self.state.borrow().probe.clone().unwrap_or_default()
    }

    fn needed_bytes(&self) -> u64 {
        self.probe().source_bytes + HEADROOM
    }

    fn disk(&self, path: &str) -> Option<Disk> {
        self.probe().disks.into_iter().find(|d| d.path == path)
    }

    fn partition(&self, path: &str) -> Option<Partition> {
        self.probe().disks.into_iter().flat_map(|d| d.partitions).find(|p| p.path == path)
    }

    fn install_target(&self) -> Option<ExistingInstall> {
        let s = self.state.borrow();
        let dev = s.reinstall.clone()?;
        s.probe.as_ref()?.installs.iter().find(|i| i.device == dev).cloned()
    }

    /// Why MAN cannot go into the free space next to Windows on a disk.
    fn alongside_problem(&self, d: &Disk) -> Option<String> {
        let need = self.needed_bytes() + BOOT_SIZE;
        if d.read_only {
            Some("Read-only".into())
        } else if !d.partitions.iter().any(|p| p.is_esp) {
            Some(tr("No EFI system partition").into())
        } else {
            match d.largest_free() {
                Some(f) if f.size >= need => None,
                Some(f) => Some(trf("Only {free} free, MAN needs {need}. Make more room with DUHACH! in Windows.", &[("free", &human_size(f.size)), ("need", &human_size(need))])),
                None => Some(tr("No free space. Start DUHACH! in Windows to make room for MAN.").into()),
            }
        }
    }

    /// Why a disk cannot be the target of "erase and install", if it can't.
    fn disk_problem(&self, d: &Disk) -> Option<String> {
        let overhead = (1u64 << 30) + if self.state.borrow().encrypt { 1 << 30 } else { 0 };
        if d.is_live {
            Some(tr("MAN is running from this disk").into())
        } else if d.read_only {
            Some("Read-only".into())
        } else if d.busy {
            Some(tr("In use: a partition is mounted").into())
        } else if d.size < MIN_DISK.max(self.needed_bytes() + overhead) {
            Some(trf("Too small, MAN needs {need}", &[("need", &human_size(MIN_DISK.max(self.needed_bytes() + overhead)))]))
        } else {
            None
        }
    }
}

pub fn run(app: &adw::Application) {
    load_css();
    crate::langui::load_css();
    let window = adw::ApplicationWindow::builder().application(app).title(tr("Install MAN")).default_width(880).default_height(700).build();
    let nav = adw::NavigationView::new();
    let toasts = adw::ToastOverlay::new();
    toasts.set_child(Some(&nav));
    window.set_content(Some(&toasts));
    let ctx: C = Rc::new(Ctx {
        window: window.clone(),
        nav,
        toasts,
        state: RefCell::new(State { timezone: "Etc/UTC".into(), keyboard: "us".into(), ..Default::default() }),
        busy: Cell::new(false),
        timezones: Rc::new(timezones()),
        keyboards: Rc::new(keyboards()),
    });
    {
        let ctx = ctx.clone();
        window.connect_close_request(move |_| {
            if ctx.busy.get() {
                ctx.toast(tr("MAN is being installed. Please wait until it finishes."));
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
    }
    ctx.nav.add(&welcome_page(&ctx));
    window.present();
}

// ───────────────────────────── Welcome ─────────────────────────────

/// "🌐 English (United States)": change the live session's language.
fn language_button(ctx: &C) -> gtk::Button {
    let current = crate::langui::system_language();
    let content = adw::ButtonContent::builder().icon_name("preferences-desktop-locale-symbolic").label(current.native).build();
    let button = gtk::Button::builder().child(&content).halign(gtk::Align::Center).margin_top(6).tooltip_text(tr("Language")).build();
    button.add_css_class("flat");
    let ctx = ctx.clone();
    button.connect_clicked(move |_| language_dialog(&ctx));
    button
}

fn language_dialog(ctx: &C) {
    let current = crate::langui::system_language();
    let chosen = Rc::new(RefCell::new(current.locale.to_string()));
    let dialog = adw::Dialog::builder().title(tr("Language")).content_width(560).content_height(680).build();
    let hero = crate::langui::hero(current.welcome);
    let apply = pill(tr("_Restart in This Language"), Some("suggested-action"));
    apply.set_sensitive(false);
    let chooser = {
        let chosen = chosen.clone();
        let title = hero.title.clone();
        let apply = apply.clone();
        crate::langui::language_list(current.locale, move |l| {
            *chosen.borrow_mut() = l.locale.to_string();
            crate::langui::set_hero_title(&title, l.welcome);
            apply.set_sensitive(l.locale != current.locale);
        })
    };
    let note = label(tr("The live session restarts in the chosen language, and MAN is installed in it."), &["dim-label", "caption"]);
    let top = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(10).build();
    top.append(&hero.widget);
    top.append(&adw::Clamp::builder().maximum_size(480).child(&chooser.search).margin_start(18).margin_end(18).build());
    let list_clamp = adw::Clamp::builder().maximum_size(480).child(&chooser.list).margin_start(18).margin_end(18).margin_top(4).margin_bottom(12).build();
    let scroll = gtk::ScrolledWindow::builder().hscrollbar_policy(gtk::PolicyType::Never).vexpand(true).child(&list_clamp).build();
    let content = gtk::Box::builder().orientation(gtk::Orientation::Vertical).build();
    content.append(&top);
    content.append(&scroll);
    let tv = adw::ToolbarView::new();
    tv.add_top_bar(&adw::HeaderBar::builder().show_title(false).build());
    tv.set_content(Some(&content));
    let bar = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(6).margin_top(10).margin_bottom(18).margin_start(18).margin_end(18).build();
    bar.append(&note);
    bar.append(&apply);
    tv.add_bottom_bar(&bar);
    dialog.set_child(Some(&tv));
    if let Some(r) = chooser.selected.clone() {
        crate::langui::reveal(&r);
    }
    {
        let ctx = ctx.clone();
        let dialog = dialog.clone();
        apply.connect_clicked(move |b| {
            let locale = chosen.borrow().clone();
            let ctx = ctx.clone();
            let dialog = dialog.clone();
            b.set_sensitive(false);
            glib::spawn_future_local(async move {
                match backend::call(&Request::SetLiveLanguage { locale }, |_| {}).await {
                    // The helper restarts the session; the installer comes
                    // back after the automatic login.
                    Ok(_) => {
                        dialog.close();
                    }
                    Err(f) => ctx.toast(&f.message),
                }
            });
        });
    }
    dialog.present(Some(&ctx.window));
}

fn welcome_page(ctx: &C) -> adw::NavigationPage {
    let status = adw::StatusPage::builder()
        .icon_name("com.man.Installer")
        .title(tr("Welcome to MAN"))
        .description(tr("A fast, open and fully customizable GNOME system.\nInstall it on this computer, or keep exploring the live session."))
        .vexpand(true)
        .build();
    let buttons = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(12).halign(gtk::Align::Center).build();
    let install = pill(tr("_Install MAN…"), Some("suggested-action"));
    install.set_width_request(260);
    let try_btn = pill(tr("_Keep Trying MAN"), None);
    try_btn.set_width_request(260);
    let spinner = adw::Spinner::builder().height_request(24).build();
    let status_label = label(tr("Looking for disks…"), &["dim-label"]);
    let busy_box = gtk::Box::builder().spacing(8).halign(gtk::Align::Center).build();
    busy_box.append(&spinner);
    busy_box.append(&status_label);
    let banner = label("", &["error"]);
    banner.set_visible(false);
    buttons.append(&install);
    buttons.append(&try_btn);
    buttons.append(&language_button(ctx));
    buttons.append(&busy_box);
    buttons.append(&banner);
    status.set_child(Some(&buttons));
    install.set_sensitive(false);

    {
        let ctx = ctx.clone();
        install.connect_clicked(move |_| ctx.nav.push(&destination_page(&ctx)));
    }
    {
        let ctx = ctx.clone();
        try_btn.connect_clicked(move |_| ctx.window.close());
    }

    let tv = adw::ToolbarView::new();
    let header = adw::HeaderBar::builder().show_title(false).build();
    tv.add_top_bar(&header);
    tv.set_content(Some(&status));
    let page = adw::NavigationPage::builder().title(tr("Welcome")).tag("welcome").child(&tv).build();

    let ctx = ctx.clone();
    glib::spawn_future_local(async move {
        let mut probe = None;
        let res = backend::call(&Request::Probe, |ev| {
            if let Event::Probe { probe: p } = ev {
                probe = Some(p.clone());
            }
        })
        .await;
        busy_box.set_visible(false);
        match (res, probe) {
            (Ok(_), Some(p)) => {
                if !p.uefi {
                    banner.set_label(tr("This computer was started in legacy BIOS mode. MAN can only be installed on UEFI computers: restart and choose the UEFI boot option."));
                    banner.set_visible(true);
                } else {
                    install.set_sensitive(true);
                    install.grab_focus();
                }
                let mut s = ctx.state.borrow_mut();
                s.timezone = p.timezone.clone();
                s.keyboard = p.keyboard.clone();
                s.probe = Some(p);
                drop(s);
                select_defaults(&ctx);
            }
            (Err(f), _) => {
                banner.set_label(&format!("{} {}", f.message, f.detail));
                banner.set_visible(true);
            }
            (Ok(_), None) => {
                banner.set_label(tr("The system helper returned no disk information."));
                banner.set_visible(true);
            }
        }
    });
    page
}

fn select_defaults(ctx: &C) {
    let probe = ctx.probe();
    let first = probe.disks.iter().find(|d| ctx.disk_problem(d).is_none()).map(|d| d.path.clone());
    let mut s = ctx.state.borrow_mut();
    s.disk = first;
    s.reinstall = probe.installs.first().map(|i| i.device.clone());
    drop(s);
    let alongside = probe.disks.iter().find(|d| d.windows && ctx.alongside_problem(d).is_none()).map(|d| d.path.clone());
    let mut s = ctx.state.borrow_mut();
    s.alongside = alongside;
    // An existing MAN system holds someone's files: offer the option that
    // keeps them first. With Windows and room next to it (DUHACH!), keep
    // Windows rather than suggesting to erase it.
    if s.reinstall.is_some() {
        s.mode = Mode::Reinstall;
    } else if s.alongside.is_some() {
        s.mode = Mode::Alongside;
    }
}

// ──────────────────────────── Destination ───────────────────────────

fn radio_row(title: &str, subtitle: &str, group: Option<&gtk::CheckButton>) -> (adw::ActionRow, gtk::CheckButton) {
    let check = gtk::CheckButton::new();
    if let Some(g) = group {
        check.set_group(Some(g));
    }
    let row = adw::ActionRow::builder().title(title).subtitle(subtitle).activatable_widget(&check).build();
    row.add_prefix(&check);
    (row, check)
}

fn boxed_list() -> gtk::ListBox {
    let l = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).build();
    l.add_css_class("boxed-list");
    l
}

fn disk_icon(d: &Disk) -> &'static str {
    if d.removable || d.transport == "usb" { "drive-removable-media-symbolic" } else { "drive-harddisk-symbolic" }
}

fn disk_subtitle(d: &Disk) -> String {
    let mut parts = vec![human_size(d.size), d.path.clone()];
    if !d.transport.is_empty() {
        parts.push(d.transport.to_uppercase());
    }
    parts.join("  ·  ")
}

fn partition_title(p: &Partition) -> String {
    let fs = p.fstype.clone().unwrap_or_else(|| "unformatted".into());
    let mut t = format!("{}  ·  {}  ·  {}", p.path, human_size(p.size), fs);
    if let Some(l) = &p.label {
        t += &format!("  ·  “{l}”");
    }
    if p.is_esp {
        t += tr("  ·  EFI");
    }
    t
}

fn destination_page(ctx: &C) -> adw::NavigationPage {
    let prefs = adw::PreferencesPage::new();
    let probe = ctx.probe();
    let next = pill(tr("_Next"), Some("suggested-action"));
    let hint = label("", &["dim-label", "caption"]);
    let validate: Rc<RefCell<Box<dyn Fn()>>> = Rc::new(RefCell::new(Box::new(|| {})));

    // Installation type.
    let type_group = adw::PreferencesGroup::builder().title(tr("Installation Type")).build();
    let has_windows = probe.disks.iter().any(|d| d.windows && !d.is_live);
    let (erase_row, erase_check) = radio_row(
        tr("Erase Disk and Install MAN"),
        if has_windows { tr("Everything on the chosen disk is deleted, Windows included.") } else { tr("Recommended. Everything on the chosen disk is deleted.") },
        None,
    );
    let (al_row, al_check) = radio_row(
        tr("Install Alongside Windows"),
        tr("Recommended. MAN uses the free space next to Windows; Windows and its files stay. Choose the system at every start."),
        Some(&erase_check),
    );
    al_row.add_suffix(&icon("computer-symbolic"));
    al_row.set_visible(has_windows);
    erase_row.add_suffix(&icon("drive-harddisk-symbolic"));
    let (re_row, re_check) = radio_row(
        tr("Reinstall MAN"),
        tr("Replaces the system but keeps personal files in /home. The current system is saved as a snapshot."),
        Some(&erase_check),
    );
    re_row.add_suffix(&icon("view-refresh-symbolic"));
    re_row.set_visible(!probe.installs.is_empty());
    let (man_row, man_check) = radio_row(tr("Custom Partitions"), tr("For experts: choose partitions yourself, or create them with Disks."), Some(&erase_check));
    man_row.add_suffix(&icon("applications-engineering-symbolic"));
    type_group.add(&al_row);
    type_group.add(&erase_row);
    type_group.add(&re_row);
    type_group.add(&man_row);
    prefs.add(&type_group);

    // Erase: disk list.
    let disk_group = adw::PreferencesGroup::builder().title(tr("Disk")).description(tr("Choose the disk to erase. Disks that cannot be used are shown with the reason.")).build();
    let disk_list = boxed_list();
    // Grouping with a hidden anchor draws radio buttons even for one disk.
    let first_check: Option<gtk::CheckButton> = Some(gtk::CheckButton::new());
    for d in &probe.disks {
        let problem = ctx.disk_problem(d);
        let subtitle = match &problem {
            Some(p) => format!("{}\n{p}", disk_subtitle(d)),
            None => disk_subtitle(d),
        };
        let (row, check) = radio_row(&glib::markup_escape_text(&d.model), &glib::markup_escape_text(&subtitle), first_check.as_ref());
        row.add_suffix(&icon(disk_icon(d)));
        row.set_sensitive(problem.is_none());
        check.set_active(ctx.state.borrow().disk.as_deref() == Some(d.path.as_str()));
        let path = d.path.clone();
        let ctx2 = ctx.clone();
        let v = validate.clone();
        check.connect_toggled(move |c| {
            if c.is_active() {
                ctx2.state.borrow_mut().disk = Some(path.clone());
                (v.borrow())();
            }
        });
        disk_list.append(&row);
    }
    if probe.disks.is_empty() {
        disk_list.append(&adw::ActionRow::builder().title(tr("No disks were found")).build());
    }
    disk_group.add(&disk_list);
    prefs.add(&disk_group);

    // Alongside: disks with Windows and the free space next to it.
    let al_group = adw::PreferencesGroup::builder()
        .title(tr("Free Space Next to Windows"))
        .description(tr("Windows' partitions are not touched. MAN shares the EFI system partition and adds Windows to its start menu."))
        .build();
    let al_list = boxed_list();
    let al_anchor: Option<gtk::CheckButton> = Some(gtk::CheckButton::new());
    for d in probe.disks.iter().filter(|d| d.windows && !d.is_live) {
        let problem = ctx.alongside_problem(d);
        let free = d.largest_free().map(|f| trf("{size} free", &[("size", &human_size(f.size))])).unwrap_or_else(|| tr("no free space").into());
        let subtitle = match &problem {
            Some(p) => format!("{}  ·  {free}\n{p}", disk_subtitle(d)),
            None => format!("{}  ·  {free}", disk_subtitle(d)),
        };
        let (row, check) = radio_row(&glib::markup_escape_text(&trf("{model} (Windows)", &[("model", &d.model)])), &glib::markup_escape_text(&subtitle), al_anchor.as_ref());
        row.add_suffix(&icon(disk_icon(d)));
        row.set_sensitive(problem.is_none());
        check.set_active(ctx.state.borrow().alongside.as_deref() == Some(d.path.as_str()));
        let path = d.path.clone();
        let ctx2 = ctx.clone();
        let v = validate.clone();
        check.connect_toggled(move |c| {
            if c.is_active() {
                ctx2.state.borrow_mut().alongside = Some(path.clone());
                (v.borrow())();
            }
        });
        al_list.append(&row);
    }
    al_group.add(&al_list);
    prefs.add(&al_group);

    // Reinstall: existing installations.
    let re_group = adw::PreferencesGroup::builder().title(tr("Installation to Replace")).build();
    let re_list = boxed_list();
    re_group.add(&re_list);
    prefs.add(&re_group);
    let fill_reinstall: Rc<RefCell<Box<dyn Fn()>>> = Rc::new(RefCell::new(Box::new(|| {})));
    {
        let ctx = ctx.clone();
        let re_list = re_list.clone();
        let v = validate.clone();
        let fill_self = fill_reinstall.clone();
        *fill_reinstall.borrow_mut() = Box::new(move || {
            re_list.remove_all();
            let group: Option<gtk::CheckButton> = Some(gtk::CheckButton::new());
            for inst in ctx.probe().installs {
                let model = ctx.disk(&inst.disk).map(|d| d.model).unwrap_or_else(|| inst.disk.clone());
                let mut sub = vec![inst.device.clone()];
                if inst.encrypted {
                    sub.push("encrypted".into());
                }
                if inst.inspected {
                    if inst.home_dirs.is_empty() {
                        sub.push(tr("no home folders").into());
                    } else {
                        sub.push(trf("home folders: {list}", &[("list", &inst.home_dirs.join(", "))]));
                    }
                    sub.push(trf("snapshots: {n}", &[("n", &inst.snapshots)]));
                } else {
                    sub.push("locked".into());
                }
                let (row, check) = radio_row(&glib::markup_escape_text(&trf("MAN on {model}", &[("model", &model)])), &glib::markup_escape_text(&sub.join("  ·  ")), group.as_ref());
                row.add_suffix(&icon(if inst.encrypted { "channel-secure-symbolic" } else { "computer-symbolic" }));
                check.set_active(ctx.state.borrow().reinstall.as_deref() == Some(inst.device.as_str()));
                let dev = inst.device.clone();
                let ctx2 = ctx.clone();
                let v2 = v.clone();
                check.connect_toggled(move |c| {
                    if c.is_active() {
                        ctx2.state.borrow_mut().reinstall = Some(dev.clone());
                        (v2.borrow())();
                    }
                });
                re_list.append(&row);
                if inst.encrypted && !inst.inspected {
                    let pass = adw::PasswordEntryRow::builder().title(trf("Passphrase for {device}", &[("device", &inst.device)])).build();
                    let unlock = gtk::Button::builder().label(tr("Unlock")).valign(gtk::Align::Center).build();
                    unlock.add_css_class("suggested-action");
                    pass.add_suffix(&unlock);
                    let ctx3 = ctx.clone();
                    let dev = inst.device.clone();
                    let fill = fill_self.clone();
                    let pass2 = pass.clone();
                    let do_unlock = Rc::new(move || {
                        let ctx = ctx3.clone();
                        let dev = dev.clone();
                        let fill = fill.clone();
                        let passphrase = pass2.text().to_string();
                        pass2.set_sensitive(false);
                        let pass3 = pass2.clone();
                        glib::spawn_future_local(async move {
                            let mut found = None;
                            let res = backend::call(&Request::Inspect { device: dev.clone(), passphrase: passphrase.clone() }, |ev| {
                                if let Event::Inspect { install } = ev {
                                    found = Some(install.clone());
                                }
                            })
                            .await;
                            pass3.set_sensitive(true);
                            match (res, found) {
                                (Ok(_), Some(inst)) => {
                                    let mut s = ctx.state.borrow_mut();
                                    if let Some(p) = s.probe.as_mut() {
                                        if let Some(slot) = p.installs.iter_mut().find(|i| i.device == inst.device) {
                                            *slot = inst;
                                        }
                                    }
                                    s.reinstall = Some(dev);
                                    s.reinstall_pass = passphrase;
                                    drop(s);
                                    (fill.borrow())();
                                }
                                (Err(f), _) => ctx.toast(&f.message),
                                _ => ctx.toast(tr("The installation could not be inspected.")),
                            }
                        });
                    });
                    let du = do_unlock.clone();
                    unlock.connect_clicked(move |_| du());
                    pass.connect_entry_activated(move |_| do_unlock());
                    re_list.append(&pass);
                }
            }
            (v.borrow())();
        });
    }
    (fill_reinstall.borrow())();

    // Manual: partition choices.
    let man_group = adw::PreferencesGroup::builder()
        .title(tr("Partitions"))
        .description(tr("The system partition is formatted with Btrfs and holds @system, @home and @snapshots. Partitions on the disk MAN is running from are not listed."))
        .build();
    let disks_btn = gtk::Button::builder().icon_name("drive-multidisk-symbolic").tooltip_text(tr("Open Disks to create or resize partitions")).valign(gtk::Align::Center).build();
    disks_btn.add_css_class("flat");
    let refresh_btn = gtk::Button::builder().icon_name("view-refresh-symbolic").tooltip_text(tr("Refresh the partition list")).valign(gtk::Align::Center).build();
    refresh_btn.add_css_class("flat");
    let suffix = gtk::Box::builder().spacing(6).build();
    suffix.append(&disks_btn);
    suffix.append(&refresh_btn);
    man_group.set_header_suffix(Some(&suffix));
    let parts: Vec<Partition> = probe.disks.iter().filter(|d| !d.is_live).flat_map(|d| d.partitions.clone()).collect();
    let parts = Rc::new(parts);
    let names: Vec<String> = std::iter::once(tr("Choose a partition").to_string()).chain(parts.iter().map(partition_title)).collect();
    let name_refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let combo = |title: &str, current: &Option<String>| {
        let row = adw::ComboRow::builder().title(title).model(&gtk::StringList::new(&name_refs)).build();
        let idx = current.as_ref().and_then(|c| parts.iter().position(|p| &p.path == c)).map(|i| i as u32 + 1).unwrap_or(0);
        row.set_selected(idx);
        row
    };
    let (esp_now, root_now, boot_now) = {
        let s = ctx.state.borrow();
        (s.esp.clone(), s.root.clone(), s.boot.clone())
    };
    let esp_row = combo(tr("EFI System Partition"), &esp_now);
    let format_esp_row = adw::SwitchRow::builder().title(tr("Format EFI System Partition")).subtitle(tr("Leave off to keep boot loaders of other systems")).build();
    format_esp_row.set_active(ctx.state.borrow().format_esp);
    let root_row = combo(tr("System Partition (formatted as Btrfs)"), &root_now);
    let boot_row = combo(tr("Boot Partition (needed with encryption)"), &boot_now);
    for r in [&esp_row, &root_row, &boot_row] {
        let ctx2 = ctx.clone();
        let parts = parts.clone();
        let v = validate.clone();
        let which = if r == &esp_row { 0 } else if r == &root_row { 1 } else { 2 };
        let fmt = format_esp_row.clone();
        r.connect_selected_notify(move |r| {
            let sel = r.selected();
            let val = (sel > 0).then(|| parts[sel as usize - 1].clone());
            let mut s = ctx2.state.borrow_mut();
            match which {
                0 => {
                    // Suggest formatting only for partitions that are not FAT yet.
                    let needs = val.as_ref().map(|p| p.fstype.as_deref() != Some("vfat")).unwrap_or(false);
                    s.esp = val.map(|p| p.path);
                    s.format_esp = needs;
                    drop(s);
                    fmt.set_active(needs);
                }
                1 => s.root = val.map(|p| p.path),
                _ => s.boot = val.map(|p| p.path),
            }
            (v.borrow())();
        });
    }
    {
        let ctx2 = ctx.clone();
        let v = validate.clone();
        format_esp_row.connect_active_notify(move |r| {
            ctx2.state.borrow_mut().format_esp = r.is_active();
            (v.borrow())();
        });
    }
    man_group.add(&esp_row);
    man_group.add(&format_esp_row);
    man_group.add(&root_row);
    man_group.add(&boot_row);
    prefs.add(&man_group);
    disks_btn.connect_clicked(|_| spawn_detached(&["gnome-disks"]));
    {
        let ctx = ctx.clone();
        refresh_btn.connect_clicked(move |_| {
            let ctx = ctx.clone();
            glib::spawn_future_local(async move {
                let mut probe = None;
                if backend::call(&Request::Probe, |ev| {
                    if let Event::Probe { probe: p } = ev {
                        probe = Some(p.clone());
                    }
                })
                .await
                .is_ok()
                {
                    if let Some(p) = probe {
                        ctx.state.borrow_mut().probe = Some(p);
                        // Rebuild this page with the new partition list.
                        ctx.nav.pop();
                        ctx.nav.push(&destination_page(&ctx));
                        ctx.toast(tr("Partition list refreshed"));
                    }
                }
            });
        });
    }

    // Encryption.
    let sec_group = adw::PreferencesGroup::builder().title(tr("Security")).build();
    let enc_row = adw::ExpanderRow::builder()
        .title(tr("Encrypt the System"))
        .subtitle(tr("Protects your data if the computer is lost or stolen. A passphrase is asked at every start."))
        .show_enable_switch(true)
        .build();
    let pass1 = adw::PasswordEntryRow::builder().title(tr("Passphrase")).build();
    let pass2 = adw::PasswordEntryRow::builder().title(tr("Confirm Passphrase")).build();
    let enc_note = adw::ActionRow::builder()
        .title(tr("Keep this passphrase safe"))
        .subtitle(tr("If it is lost, the data cannot be recovered by anyone. The kernel and boot files stay on a small unencrypted boot partition."))
        .build();
    enc_note.add_prefix(&icon("dialog-warning-symbolic"));
    enc_row.add_row(&pass1);
    enc_row.add_row(&pass2);
    enc_row.add_row(&enc_note);
    {
        let s = ctx.state.borrow();
        enc_row.set_enable_expansion(s.encrypt);
        enc_row.set_expanded(s.encrypt);
        pass1.set_text(&s.passphrase);
        pass2.set_text(&s.passphrase2);
    }
    sec_group.add(&enc_row);
    prefs.add(&sec_group);
    {
        let ctx2 = ctx.clone();
        let v = validate.clone();
        enc_row.connect_enable_expansion_notify(move |r| {
            ctx2.state.borrow_mut().encrypt = r.enables_expansion();
            (v.borrow())();
        });
        let ctx2 = ctx.clone();
        let v = validate.clone();
        pass1.connect_changed(move |e| {
            ctx2.state.borrow_mut().passphrase = e.text().into();
            (v.borrow())();
        });
        let ctx2 = ctx.clone();
        let v = validate.clone();
        pass2.connect_changed(move |e| {
            ctx2.state.borrow_mut().passphrase2 = e.text().into();
            (v.borrow())();
        });
    }

    // Mode switching shows the matching group.
    let mode_now = ctx.state.borrow().mode;
    match mode_now {
        Mode::Erase => erase_check.set_active(true),
        Mode::Reinstall => re_check.set_active(true),
        Mode::Manual => man_check.set_active(true),
        Mode::Alongside => al_check.set_active(true),
    }
    for (check, mode) in [(&erase_check, Mode::Erase), (&re_check, Mode::Reinstall), (&man_check, Mode::Manual), (&al_check, Mode::Alongside)] {
        let ctx2 = ctx.clone();
        let v = validate.clone();
        check.connect_toggled(move |c| {
            if c.is_active() {
                ctx2.state.borrow_mut().mode = mode;
                (v.borrow())();
            }
        });
    }

    // Validation drives visibility, the hint and the Next button.
    {
        let ctx = ctx.clone();
        let next = next.clone();
        let hint = hint.clone();
        let (disk_group, re_group, man_group, sec_group, boot_row, pass2) = (disk_group.clone(), re_group.clone(), man_group.clone(), sec_group.clone(), boot_row.clone(), pass2.clone());
        let al_group = al_group.clone();
        *validate.borrow_mut() = Box::new(move || {
            let s = ctx.state.borrow();
            disk_group.set_visible(s.mode == Mode::Erase);
            al_group.set_visible(s.mode == Mode::Alongside);
            re_group.set_visible(s.mode == Mode::Reinstall);
            man_group.set_visible(s.mode == Mode::Manual);
            sec_group.set_visible(s.mode != Mode::Reinstall);
            boot_row.set_visible(s.encrypt);
            let enc_problem = if s.encrypt && s.mode != Mode::Reinstall {
                if s.passphrase.chars().count() < 8 {
                    Some(tr("The passphrase needs at least 8 characters."))
                } else if s.passphrase != s.passphrase2 {
                    Some(tr("The passphrases do not match."))
                } else {
                    None
                }
            } else {
                None
            };
            if s.encrypt && !s.passphrase2.is_empty() && s.passphrase != s.passphrase2 {
                pass2.add_css_class("error");
            } else {
                pass2.remove_css_class("error");
            }
            let problem: Option<String> = match s.mode {
                Mode::Erase => match s.disk.as_ref().and_then(|p| ctx.disk(p)) {
                    None => Some(tr("Choose a disk.").into()),
                    Some(d) => {
                        drop(s);
                        let p = ctx.disk_problem(&d);
                        let s = ctx.state.borrow();
                        let _ = &s;
                        p.or(enc_problem.map(String::from))
                    }
                },
                Mode::Alongside => match s.alongside.clone().and_then(|p| ctx.disk(&p)) {
                    None => Some(tr("Choose the disk with Windows.").into()),
                    Some(d) => {
                        drop(s);
                        ctx.alongside_problem(&d).or(enc_problem.map(String::from))
                    }
                },
                Mode::Reinstall => {
                    drop(s);
                    match ctx.install_target() {
                        None => Some(tr("Choose the installation to replace.").into()),
                        Some(i) if i.encrypted && !i.inspected => Some(tr("Unlock the encrypted installation first.").into()),
                        Some(i) if !i.has_system && !i.has_home => Some(tr("This partition holds no MAN system.").into()),
                        _ => None,
                    }
                }
                Mode::Manual => {
                    let need = ctx.needed_bytes();
                    let p = if s.esp.is_none() {
                        Some(tr("Choose an EFI system partition.").into())
                    } else if s.root.is_none() {
                        Some(tr("Choose the system partition.").into())
                    } else if s.esp == s.root {
                        Some(tr("The EFI and system partitions must be different.").into())
                    } else if s.encrypt && (s.boot.is_none() || s.boot == s.root || s.boot == s.esp) {
                        Some(tr("Encryption needs a separate boot partition.").into())
                    } else {
                        let esp = s.esp.clone().unwrap();
                        let root = s.root.clone().unwrap();
                        let fmt = s.format_esp;
                        drop(s);
                        let esp_p = ctx.partition(&esp);
                        let root_p = ctx.partition(&root);
                        if root_p.as_ref().map(|p| p.size < need).unwrap_or(true) {
                            Some(trf("The system partition must be at least {size}.", &[("size", &human_size(need))]))
                        } else if !fmt && esp_p.and_then(|p| p.fstype).as_deref() != Some("vfat") {
                            Some(tr("The EFI partition is not FAT formatted; turn on formatting.").into())
                        } else if root_p.map(|p| !p.mountpoints.is_empty()).unwrap_or(false) {
                            Some(tr("The system partition is mounted; unmount it in Disks first.").into())
                        } else {
                            enc_problem.map(String::from)
                        }
                    };
                    p
                }
            };
            next.set_sensitive(problem.is_none());
            hint.set_label(problem.as_deref().unwrap_or(""));
            hint.set_visible(problem.is_some());
        });
    }
    (validate.borrow())();

    {
        let ctx = ctx.clone();
        next.connect_clicked(move |_| ctx.nav.push(&region_page(&ctx)));
    }
    let bottom = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(6).build();
    bottom.append(&hint);
    bottom.append(&next);
    page(tr("Where Should MAN Be Installed?"), "destination", &prefs, Some(bottom.upcast_ref()))
}

// ───────────────────────────── Region ──────────────────────────────

fn region_page(ctx: &C) -> adw::NavigationPage {
    let prefs = adw::PreferencesPage::new();
    let group = adw::PreferencesGroup::builder().title(tr("Time and Keyboard")).build();
    let (tz, kb) = {
        let s = ctx.state.borrow();
        (s.timezone.clone(), s.keyboard.clone())
    };
    let tz_row = adw::ActionRow::builder().title(tr("Time Zone")).subtitle(glib::markup_escape_text(&tz)).activatable(true).build();
    tz_row.add_prefix(&icon("preferences-system-time-symbolic"));
    tz_row.add_suffix(&icon("go-next-symbolic"));
    let kb_row = adw::ActionRow::builder()
        .title(tr("Keyboard Layout"))
        .subtitle(glib::markup_escape_text(&title_for(&ctx.keyboards, &kb)))
        .activatable(true)
        .build();
    kb_row.add_prefix(&icon("input-keyboard-symbolic"));
    kb_row.add_suffix(&icon("go-next-symbolic"));
    group.add(&tz_row);
    group.add(&kb_row);
    prefs.add(&group);

    let test = adw::PreferencesGroup::builder().title(tr("Test Your Keyboard")).description(tr("Type here to check that your keys produce the right characters.")).build();
    test.add(&adw::EntryRow::builder().title(tr("Type here")).build());
    prefs.add(&test);

    {
        let ctx = ctx.clone();
        tz_row.connect_activated(move |row| {
            let ctx2 = ctx.clone();
            let row2 = row.clone();
            let current = ctx.state.borrow().timezone.clone();
            chooser(&ctx.window, tr("Time Zone"), ctx.timezones.clone(), &current, move |id| {
                ctx2.state.borrow_mut().timezone = id.to_string();
                row2.set_subtitle(&glib::markup_escape_text(id));
            });
        });
    }
    {
        let ctx = ctx.clone();
        kb_row.connect_activated(move |row| {
            let ctx2 = ctx.clone();
            let row2 = row.clone();
            let current = ctx.state.borrow().keyboard.clone();
            chooser(&ctx.window, tr("Keyboard Layout"), ctx.keyboards.clone(), &current, move |id| {
                ctx2.state.borrow_mut().keyboard = id.to_string();
                use_layout_now(id);
                row2.set_subtitle(&glib::markup_escape_text(&title_for(&ctx2.keyboards, id)));
            });
        });
    }
    let next = pill(tr("_Next"), Some("suggested-action"));
    {
        let ctx = ctx.clone();
        next.connect_clicked(move |_| ctx.nav.push(&summary_page(&ctx)));
    }
    page(tr("Time Zone and Keyboard"), "region", &prefs, Some(next.upcast_ref()))
}

/// Switch the live session to the chosen layout, so the passwords typed
/// next and the test field use the same keys as the installed system.
fn use_layout_now(id: &str) {
    const SCHEMA: &str = "org.gnome.desktop.input-sources";
    let available = gio::SettingsSchemaSource::default().and_then(|s| s.lookup(SCHEMA, true)).is_some();
    if available {
        let settings = gio::Settings::new(SCHEMA);
        let _ = settings.set_value("sources", &vec![("xkb".to_string(), id.to_string())].to_variant());
    }
}

// ───────────────────────────── Summary ─────────────────────────────

fn info_row(title: &str, value: &str, icon_name: &str) -> adw::ActionRow {
    let row = adw::ActionRow::builder().title(title).subtitle(glib::markup_escape_text(value)).subtitle_selectable(true).build();
    row.add_css_class("property");
    row.add_prefix(&icon(icon_name));
    row
}

fn build_plan(ctx: &C) -> Result<(Plan, Disk), String> {
    let s = ctx.state.borrow();
    let mut plan = Plan {
        mode: s.mode,
        format_esp: s.format_esp,
        encrypt: s.encrypt && s.mode != Mode::Reinstall,
        // The account and computer name are chosen in MAN Setup on first start.
        user: None,
        root_password: None,
        hostname: "man".into(),
        timezone: s.timezone.clone(),
        keyboard: s.keyboard.clone(),
        // The live session's language, as chosen on the first page.
        language: s.probe.as_ref().and_then(|p| p.language.clone()).or_else(|| Some(crate::langui::system_language().locale.into())),
        ..Default::default()
    };
    if plan.encrypt {
        plan.passphrase = Some(s.passphrase.clone());
    }
    match s.mode {
        Mode::Erase => plan.disk = s.disk.clone().ok_or(tr("No disk chosen"))?,
        Mode::Alongside => plan.disk = s.alongside.clone().ok_or(tr("No disk chosen"))?,
        Mode::Manual => {
            plan.esp = s.esp.clone();
            plan.root = s.root.clone();
            plan.boot = if plan.encrypt { s.boot.clone() } else { None };
            let root = s.root.clone().ok_or(tr("No system partition chosen"))?;
            drop(s);
            plan.disk = ctx.partition(&root).map(|p| p.disk).ok_or(tr("The system partition disappeared"))?;
        }
        Mode::Reinstall => {
            let pass = s.reinstall_pass.clone();
            drop(s);
            let inst = ctx.install_target().ok_or(tr("No installation chosen"))?;
            plan.disk = inst.disk.clone();
            plan.root = Some(inst.device.clone());
            plan.esp = inst.esp.clone();
            plan.boot = inst.boot.clone();
            plan.encrypt = inst.encrypted;
            if inst.encrypted {
                plan.passphrase = Some(pass);
            }
        }
    }
    let disk = ctx.disk(&plan.disk).ok_or(tr("The disk disappeared"))?;
    plan.confirm = confirm_token(&disk);
    Ok((plan, disk))
}

fn summary_page(ctx: &C) -> adw::NavigationPage {
    let prefs = adw::PreferencesPage::new();
    let (plan, disk) = match build_plan(ctx) {
        Ok(x) => x,
        Err(e) => {
            ctx.toast(&e);
            return page(tr("Review"), "summary", &adw::StatusPage::builder().title(e).build(), None);
        }
    };
    let dest = adw::PreferencesGroup::builder().title(tr("Destination")).build();
    let mode = match plan.mode {
        Mode::Erase => tr("Erase disk and install"),
        Mode::Manual => tr("Custom partitions"),
        Mode::Reinstall => tr("Reinstall, keeping /home"),
        Mode::Alongside => tr("Alongside Windows"),
    };
    dest.add(&info_row(tr("Installation Type"), mode, "drive-harddisk-symbolic"));
    dest.add(&info_row(tr("Disk"), &format!("{} ({}, {})", disk.model, human_size(disk.size), disk.path), "drive-harddisk-symbolic"));
    if plan.mode == Mode::Manual {
        let esp = plan.esp.clone().unwrap_or_default();
        dest.add(&info_row(tr("EFI System Partition"), &format!("{esp}{}", if plan.format_esp { tr(" (will be formatted)") } else { tr(" (kept)") }), "media-flash-symbolic"));
        dest.add(&info_row(tr("System Partition"), &trf("{partition} (will be formatted)", &[("partition", &plan.root.clone().unwrap_or_default())]), "media-flash-symbolic"));
        if let Some(b) = &plan.boot {
            dest.add(&info_row(tr("Boot Partition"), &trf("{partition} (will be formatted)", &[("partition", &b)]), "media-flash-symbolic"));
        }
    }
    if plan.mode == Mode::Reinstall {
        dest.add(&info_row(tr("Installation"), &plan.root.clone().unwrap_or_default(), "computer-symbolic"));
    }
    if plan.mode == Mode::Alongside {
        let free = disk.largest_free().map(|f| human_size(f.size)).unwrap_or_default();
        dest.add(&info_row(tr("Space for MAN"), &trf("{free} of free space (a boot partition and the system partition are created)", &[("free", &free)]), "media-flash-symbolic"));
        dest.add(&info_row(tr("Windows"), tr("Not changed. It appears in MAN's start menu; the EFI system partition is shared."), "computer-symbolic"));
    }
    dest.add(&info_row(tr("Encryption"), if plan.encrypt { tr("On (LUKS2)") } else { tr("Off") }, "channel-secure-symbolic"));
    dest.add(&info_row(
        tr("File System"),
        tr("Btrfs: @system for MAN, @home for your files, @snapshots for restore points. All share the free space."),
        "folder-symbolic",
    ));
    prefs.add(&dest);

    let sys = adw::PreferencesGroup::builder().title(tr("System")).build();
    sys.add(&info_row(
        tr("Your Account"),
        tr("Created in MAN Setup when the computer starts for the first time, together with the computer name, look and privacy settings."),
        "avatar-default-symbolic",
    ));
    sys.add(&info_row(tr("Time Zone"), &plan.timezone, "preferences-system-time-symbolic"));
    sys.add(&info_row(tr("Keyboard"), &title_for(&ctx.keyboards, &plan.keyboard), "input-keyboard-symbolic"));
    prefs.add(&sys);

    let (warning, button_label, button_class) = match plan.mode {
        Mode::Erase => (trf("All data on {model} ({path}) will be erased.", &[("model", &disk.model), ("path", &disk.path)]), tr("_Erase and Install"), "destructive-action"),
        Mode::Manual => (tr("The chosen partitions will be formatted. Other partitions are not changed.").to_string(), tr("_Format and Install"), "destructive-action"),
        Mode::Reinstall => (tr("The current system is replaced. Your files in /home are kept, and the old system is saved as a snapshot.").to_string(), tr("_Reinstall"), "suggested-action"),
        Mode::Alongside => (tr("MAN is installed into the free space. Windows and its files are not changed.").to_string(), tr("_Install"), "suggested-action"),
    };
    let warn_group = adw::PreferencesGroup::new();
    let warn = adw::ActionRow::builder().title(glib::markup_escape_text(&warning)).build();
    warn.add_prefix(&icon("dialog-warning-symbolic"));
    warn.add_css_class(if matches!(plan.mode, Mode::Reinstall | Mode::Alongside) { "accent" } else { "warning" });
    warn_group.add(&warn);
    prefs.add(&warn_group);

    let go = pill(button_label, Some(button_class));
    {
        let ctx = ctx.clone();
        go.connect_clicked(move |_| confirm_and_install(&ctx));
    }
    page(tr("Ready to Install"), "summary", &prefs, Some(go.upcast_ref()))
}

fn confirm_and_install(ctx: &C) {
    let (plan, disk) = match build_plan(ctx) {
        Ok(x) => x,
        Err(e) => return ctx.toast(&e),
    };
    let (heading, body, resp, appearance) = match plan.mode {
        Mode::Erase => (
            trf("Erase {model}?", &[("model", &disk.model)]),
            trf("Every file and partition on {path} ({size}) will be permanently deleted. This cannot be undone.", &[("path", &disk.path), ("size", &human_size(disk.size))]),
            tr("Erase and Install"),
            adw::ResponseAppearance::Destructive,
        ),
        Mode::Manual => (
            tr("Format the Chosen Partitions?").to_string(),
            tr("All data on the chosen partitions will be permanently deleted. Other partitions are not changed.").to_string(),
            tr("Format and Install"),
            adw::ResponseAppearance::Destructive,
        ),
        Mode::Alongside => (
            tr("Install MAN Alongside Windows?").to_string(),
            trf(
                "MAN goes into the {free} of free space on {model} ({path}). Windows and its files are not changed.",
                &[("free", &disk.largest_free().map(|f| human_size(f.size)).unwrap_or_default()), ("model", &disk.model), ("path", &disk.path)],
            ),
            tr("Install"),
            adw::ResponseAppearance::Suggested,
        ),
        Mode::Reinstall => (
            tr("Reinstall MAN?").to_string(),
            tr("The installed system will be replaced by a fresh copy. Personal files in /home are kept and the old system remains available as a snapshot.").to_string(),
            tr("Reinstall"),
            adw::ResponseAppearance::Suggested,
        ),
    };
    let dialog = adw::AlertDialog::new(Some(&heading), Some(&body));
    dialog.add_response("cancel", tr("_Cancel"));
    dialog.add_response("go", resp);
    dialog.set_response_appearance("go", appearance);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    let ctx = ctx.clone();
    glib::spawn_future_local(async move {
        if dialog.choose_future(Some(&ctx.window)).await == "go" {
            start_install(&ctx, plan);
        }
    });
}

// ───────────────────────────── Progress ────────────────────────────

fn start_install(ctx: &C, plan: Plan) {
    let content = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(18).valign(gtk::Align::Center).margin_start(24).margin_end(24).build();
    let hero = gtk::Image::from_icon_name("com.man.Installer");
    hero.add_css_class("hero-icon");
    let title = label(tr("Installing MAN"), &["step-title"]);
    let step_label = label(tr("Starting…"), &["dim-label"]);
    let bar = gtk::ProgressBar::builder().show_text(false).build();
    bar.add_css_class("install-progress");
    let detail = label("", &["caption", "dim-label"]);
    let log_view = gtk::TextView::builder().editable(false).cursor_visible(false).wrap_mode(gtk::WrapMode::WordChar).build();
    log_view.add_css_class("log-view");
    log_view.add_css_class("card");
    let log_scroll = gtk::ScrolledWindow::builder().min_content_height(200).child(&log_view).build();
    let expander = gtk::Expander::builder().label(tr("Details")).child(&log_scroll).build();
    for w in [hero.upcast_ref::<gtk::Widget>(), title.upcast_ref(), step_label.upcast_ref(), bar.upcast_ref(), detail.upcast_ref(), expander.upcast_ref()] {
        content.append(w);
    }
    let clamp = adw::Clamp::builder().maximum_size(560).child(&content).vexpand(true).build();
    let progress_page = page(tr("Installing"), "progress", &clamp, None);
    progress_page.set_can_pop(false);
    ctx.nav.push(&progress_page);
    ctx.busy.set(true);

    let ctx = ctx.clone();
    glib::spawn_future_local(async move {
        let buffer = log_view.buffer();
        let mut current_step = 0usize;
        let total: f64 = WEIGHTS.iter().sum();
        let res = backend::call(&Request::Install { plan }, |ev| match ev {
            Event::Step { index, total: n, title } => {
                current_step = (*index as usize).saturating_sub(1).min(WEIGHTS.len() - 1);
                step_label.set_label(&trf("Step {index} of {n}  ·  {title}", &[("index", &index), ("n", &n), ("title", &tr_dyn(title))]));
                detail.set_label("");
                let done: f64 = WEIGHTS[..current_step].iter().sum();
                bar.set_fraction(done / total);
            }
            Event::Progress { fraction, detail: d } => {
                let done: f64 = WEIGHTS[..current_step].iter().sum();
                bar.set_fraction((done + WEIGHTS[current_step] * fraction) / total);
                detail.set_label(&tr_dyn(d));
            }
            Event::Log { line } => {
                let mut end = buffer.end_iter();
                buffer.insert(&mut end, &format!("{line}\n"));
                let mark = buffer.create_mark(None, &buffer.end_iter(), false);
                log_view.scroll_mark_onscreen(&mark);
                buffer.delete_mark(&mark);
            }
            _ => {}
        })
        .await;
        ctx.busy.set(false);
        let log_text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string();
        match res {
            Ok(message) => ctx.nav.replace(&[done_page(&ctx, &message)]),
            Err(f) => {
                let p = error_page(&ctx, &f.message, &format!("{}\n\n── {} ──\n{}", f.detail, tr("Log"), log_text));
                ctx.nav.push(&p);
            }
        }
    });
}

fn done_page(ctx: &C, message: &str) -> adw::NavigationPage {
    let status = adw::StatusPage::builder()
        .icon_name("object-select-symbolic")
        .title(tr("MAN Is Installed"))
        .description(if ctx.state.borrow().mode == Mode::Alongside {
            format!(
                "{}\n{}",
                tr_dyn(message),
                tr("Restart the computer to start using MAN. A menu at every start lets you choose MAN or Windows; the temporary MAN installer partition is removed by MAN on its first start.")
            )
        } else {
            format!(
                "{}\n{}",
                tr_dyn(message),
                tr("Restart the computer to start using MAN. You can remove the installation media; if it stays connected, MAN still starts from the new disk.")
            )
        })
        .vexpand(true)
        .build();
    status.add_css_class("done-page");
    let buttons = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(12).halign(gtk::Align::Center).build();
    let restart = pill(tr("_Restart Now"), Some("suggested-action"));
    restart.set_width_request(240);
    let later = pill(tr("_Keep Trying MAN"), None);
    later.set_width_request(240);
    buttons.append(&restart);
    buttons.append(&later);
    status.set_child(Some(&buttons));
    restart.connect_clicked(|_| spawn_detached(&["systemctl", "reboot"]));
    {
        let ctx = ctx.clone();
        later.connect_clicked(move |_| ctx.window.close());
    }
    let tv = adw::ToolbarView::new();
    tv.add_top_bar(&adw::HeaderBar::builder().show_title(false).build());
    tv.set_content(Some(&status));
    let p = adw::NavigationPage::builder().title(tr("Done")).tag("done").child(&tv).can_pop(false).build();
    let restart2 = restart.clone();
    p.connect_shown(move |_| {
        restart2.grab_focus();
    });
    p
}

fn error_page(ctx: &C, message: &str, details: &str) -> adw::NavigationPage {
    let status = adw::StatusPage::builder()
        .icon_name("dialog-error-symbolic")
        .title(tr("Installation Failed"))
        .description(format!(
            "{}\n\n{}",
            tr_dyn(message),
            tr("No other disk was changed, and the installer has released the target disk. You can review the plan and try again.")
        ))
        .vexpand(true)
        .build();
    let buttons = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(12).halign(gtk::Align::Center).build();
    let back = pill(tr("_Review and Try Again"), Some("suggested-action"));
    back.set_width_request(260);
    let show = pill(tr("_Show Details"), None);
    show.set_width_request(260);
    buttons.append(&back);
    buttons.append(&show);
    status.set_child(Some(&buttons));
    {
        let ctx = ctx.clone();
        back.connect_clicked(move |_| {
            // Back to the summary; the progress page is removed from history.
            ctx.nav.pop_to_tag("summary");
        });
    }
    {
        let ctx = ctx.clone();
        let details = details.to_string();
        show.connect_clicked(move |_| details_dialog(&ctx.window, tr("Installation Log"), &details));
    }
    let tv = adw::ToolbarView::new();
    tv.add_top_bar(&adw::HeaderBar::builder().show_title(false).build());
    tv.set_content(Some(&status));
    adw::NavigationPage::builder().title(tr("Installation Failed")).tag("error").child(&tv).can_pop(false).build()
}

pub fn main() -> glib::ExitCode {
    crate::i18n::init();
    let app = adw::Application::builder().application_id("com.man.Installer").flags(gio::ApplicationFlags::default()).build();
    app.connect_activate(|app| {
        if let Some(w) = app.active_window() {
            w.present();
        } else {
            run(app);
        }
    });
    app.run()
}
