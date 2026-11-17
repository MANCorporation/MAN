//! "MAN Recovery": started from the boot menu of an installed MAN. Runs
//! full screen in a minimal mutter session (its own system, `@recovery`).
//! After an administrator's password it can restore a snapshot, reinstall
//! MAN from the newest ISO (keeping files and accounts) or open a Terminal.
use crate::backend;
use crate::i18n::{tr, trf};
use crate::net::{connect_wifi, net_status, signal_icon, wifi_networks};
use crate::ui::*;
use adw::prelude::*;
use gtk::{gdk, gio, glib};
use man_common::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const CSS: &str = r#"
.recovery-backdrop { background: radial-gradient(circle at 50% 30%, #2b2b3a 0%, #121217 70%); }
.recovery-card {
  background: var(--window-bg-color);
  color: var(--window-fg-color);
  border-radius: 28px;
  box-shadow: 0 18px 48px alpha(black, 0.5), 0 0 0 1px alpha(white, 0.06);
}
.recovery-title { font-weight: 800; font-size: 2em; }
.recovery-symbol { -gtk-icon-size: 72px; color: var(--accent-color); }
.recovery-option { padding: 10px 6px; }
.recovery-option image.option-icon { -gtk-icon-size: 32px; color: var(--accent-color); }
.recovery-done { -gtk-icon-size: 96px; color: var(--success-color); }
.recovery-failed { -gtk-icon-size: 96px; color: var(--error-color); }
.recovery-footer { color: alpha(white, 0.75); }
.recovery-footer button { color: white; }
"#;

struct Ctx {
    window: adw::ApplicationWindow,
    nav: adw::NavigationView,
    toasts: adw::ToastOverlay,
    info: RefCell<RecoveryInfo>,
    /// The administrator who unlocked Recovery, and the password: each
    /// change is checked again by the system helper.
    who: RefCell<(String, String)>,
    busy: Cell<bool>,
}
type C = Rc<Ctx>;

impl Ctx {
    fn toast(&self, t: &str) {
        self.toasts.add_toast(adw::Toast::new(t));
    }
}

fn vbox(spacing: i32) -> gtk::Box {
    gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(spacing).build()
}

fn heading(icon_name: &str, title: &str, description: &str) -> gtk::Box {
    let b = vbox(10);
    b.set_margin_bottom(18);
    let img = gtk::Image::from_icon_name(icon_name);
    img.add_css_class("recovery-symbol");
    b.append(&img);
    b.append(&label(title, &["recovery-title"]));
    if !description.is_empty() {
        b.append(&label(description, &["dim-label"]));
    }
    b
}

/// A page of the card: header (back button only), clamped content, bottom bar.
fn card_page(tag: &str, title: &str, content: &impl IsA<gtk::Widget>, bottom: Option<&gtk::Widget>) -> adw::NavigationPage {
    let tv = adw::ToolbarView::new();
    tv.add_top_bar(&adw::HeaderBar::builder().show_end_title_buttons(false).show_start_title_buttons(false).show_title(false).build());
    let clamp = adw::Clamp::builder().maximum_size(560).child(content).margin_start(24).margin_end(24).margin_bottom(12).build();
    tv.set_content(Some(&gtk::ScrolledWindow::builder().hscrollbar_policy(gtk::PolicyType::Never).vexpand(true).child(&clamp).build()));
    if let Some(b) = bottom {
        let bar = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(6).margin_top(10).margin_bottom(22).margin_start(24).margin_end(24).build();
        bar.append(b);
        tv.add_bottom_bar(&bar);
    }
    adw::NavigationPage::builder().title(title).tag(tag).child(&tv).build()
}

fn spawn(argv: &[&str]) {
    let os: Vec<&std::ffi::OsStr> = argv.iter().map(std::ffi::OsStr::new).collect();
    if let Err(e) = gio::Subprocess::newv(&os, gio::SubprocessFlags::NONE) {
        eprintln!("man-recovery: {}: {e}", argv[0]);
    }
}

fn restart() {
    spawn(&["systemctl", "reboot"]);
}

fn power_off() {
    spawn(&["systemctl", "poweroff"]);
}

fn date_of(rfc: &str) -> String {
    glib::DateTime::from_iso8601(rfc, None)
        .ok()
        .and_then(|d| d.to_local().ok())
        .and_then(|d| d.format("%x %X").ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| rfc.replace('T', " ").replace('Z', ""))
}

// ─────────────────────────── Unlock ───────────────────────────

fn unlock_page(ctx: &C) -> adw::NavigationPage {
    let content = vbox(18);
    let info = ctx.info.borrow().clone();
    content.append(&heading(
        "system-reboot-symbolic",
        tr("MAN Recovery"),
        tr("Restore your system, reinstall MAN or open a Terminal. First, an administrator of this computer signs in."),
    ));
    let list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).build();
    list.add_css_class("boxed-list");
    let users = gtk::StringList::new(&[]);
    for a in &info.admins {
        users.append(&a.real_name);
    }
    let user_row = adw::ComboRow::builder().title(tr("Administrator")).model(&users).build();
    let password = adw::PasswordEntryRow::builder().title(tr("Password")).activates_default(true).build();
    list.append(&user_row);
    list.append(&password);
    let next = pill(tr("_Continue"), Some("suggested-action"));
    let no_admins = info.admins.is_empty();
    if no_admins {
        // The installed system is missing: nothing to sign in to.
        list.set_visible(false);
        content.append(&label(tr("No installed system was found on this disk. You can reinstall MAN."), &["dim-label"]));
    }
    content.append(&list);
    let spinner = adw::Spinner::builder().visible(false).halign(gtk::Align::Center).build();
    content.append(&spinner);
    {
        let entry_for_enter = password.clone();
        let (ctx, password, user_row, spinner) = (ctx.clone(), password.clone(), user_row.clone(), spinner.clone());
        let go = Rc::new(move || {
            let (ctx, password, spinner) = (ctx.clone(), password.clone(), spinner.clone());
            let user = ctx.info.borrow().admins.get(user_row.selected() as usize).map(|a| a.name.clone()).unwrap_or_default();
            let pw = password.text().to_string();
            if ctx.busy.get() {
                return;
            }
            ctx.busy.set(true);
            spinner.set_visible(true);
            glib::spawn_future_local(async move {
                let r = backend::call(&Request::RecoveryUnlock { user: user.clone(), password: pw.clone() }, |_| {}).await;
                spinner.set_visible(false);
                ctx.busy.set(false);
                match r {
                    Ok(_) => {
                        *ctx.who.borrow_mut() = (user, pw);
                        password.set_text("");
                        ctx.nav.push(&options_page(&ctx));
                    }
                    Err(f) => {
                        ctx.toast(&f.message);
                        password.set_text("");
                        password.grab_focus();
                    }
                }
            });
        });
        let g = go.clone();
        next.connect_clicked(move |_| g());
        entry_for_enter.connect_entry_activated(move |_| go());
    }
    let page = card_page("unlock", tr("MAN Recovery"), &content, Some(next.upcast_ref()));
    if no_admins {
        let ctx = ctx.clone();
        next.connect_clicked(move |_| ctx.nav.push(&options_page(&ctx)));
    } else {
        let p = password.clone();
        page.connect_shown(move |_| {
            p.grab_focus();
        });
    }
    page
}

// ─────────────────────────── Options ───────────────────────────

fn option_row(icon_name: &str, title: &str, subtitle: &str) -> adw::ActionRow {
    let row = adw::ActionRow::builder().title(title).subtitle(subtitle).activatable(true).build();
    row.add_css_class("recovery-option");
    let img = icon(icon_name);
    img.add_css_class("option-icon");
    row.add_prefix(&img);
    row.add_suffix(&icon("go-next-symbolic"));
    row
}

fn options_page(ctx: &C) -> adw::NavigationPage {
    let content = vbox(18);
    let info = ctx.info.borrow().clone();
    let installed = if info.version.is_empty() { String::new() } else { trf("This computer has MAN {version}.", &[("version", &info.version)]) };
    content.append(&heading("system-reboot-symbolic", tr("What Would You Like to Do?"), &installed));
    let list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).build();
    list.add_css_class("boxed-list");
    let restore = option_row("document-revert-symbolic", tr("Restore a Snapshot"), tr("Your system broke? Go back to a moment when everything worked. Your files stay."));
    let reinstall = option_row("folder-download-symbolic", tr("Reinstall MAN"), tr("Download the newest MAN and install it again. Your files, accounts and settings stay."));
    let terminal = option_row("utilities-terminal-symbolic", tr("Terminal"), tr("For experts: type commands with full control of the computer."));
    restore.set_sensitive(!info.snapshots.is_empty() && !ctx.who.borrow().0.is_empty());
    if info.snapshots.is_empty() {
        restore.set_subtitle(tr("There are no snapshots on this computer."));
    }
    terminal.set_sensitive(!ctx.who.borrow().0.is_empty());
    list.append(&restore);
    list.append(&reinstall);
    list.append(&terminal);
    content.append(&list);
    {
        let ctx = ctx.clone();
        restore.connect_activated(move |_| ctx.nav.push(&snapshots_page(&ctx)));
    }
    {
        let ctx = ctx.clone();
        reinstall.connect_activated(move |_| {
            let ctx = ctx.clone();
            glib::spawn_future_local(async move {
                let st = gio::spawn_blocking(net_status).await.ok();
                if st.is_some_and(|s| s.connected) {
                    ctx.nav.push(&release_page(&ctx));
                } else {
                    ctx.nav.push(&network_page(&ctx));
                }
            });
        });
    }
    terminal.connect_activated(|_| spawn(&["kgx", "--", "sudo", "-i"]));
    card_page("options", tr("MAN Recovery"), &content, None)
}

// ─────────────────────────── Restore ───────────────────────────

fn snapshots_page(ctx: &C) -> adw::NavigationPage {
    let content = vbox(18);
    content.append(&heading("document-revert-symbolic", tr("Restore a Snapshot"), tr("Choose a moment when everything worked. The files in your home folder stay as they are now.")));
    let list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).build();
    list.add_css_class("boxed-list");
    for s in ctx.info.borrow().snapshots.iter() {
        let row = adw::ActionRow::builder().title(glib::markup_escape_text(&s.description)).subtitle(date_of(&s.created)).activatable(true).build();
        row.add_suffix(&icon("go-next-symbolic"));
        let (ctx, s) = (ctx.clone(), s.clone());
        row.connect_activated(move |_| confirm_restore(&ctx, &s));
        list.append(&row);
    }
    content.append(&list);
    card_page("snapshots", tr("Restore a Snapshot"), &content, None)
}

fn confirm_restore(ctx: &C, s: &SnapshotInfo) {
    let when = date_of(&s.created);
    let dialog = adw::AlertDialog::new(
        Some(&trf("Restore “{name}”?", &[("name", &s.description)])),
        Some(&trf("Your system goes back to how it was on {date}. Your files stay. The system as it is now is kept as a snapshot, so you can come back to it.", &[("date", &when)])),
    );
    dialog.add_response("cancel", tr("_Cancel"));
    dialog.add_response("restore", tr("_Restore"));
    dialog.set_response_appearance("restore", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("restore"));
    dialog.set_close_response("cancel");
    let (ctx, id) = (ctx.clone(), s.id.clone());
    glib::spawn_future_local(async move {
        if dialog.choose_future(Some(&ctx.window)).await != "restore" {
            return;
        }
        let (progress, set) = working_page(&ctx, "document-revert-symbolic", tr("Restoring Your System"));
        ctx.nav.push(&progress);
        let (user, password) = ctx.who.borrow().clone();
        let r = backend::call(&Request::RecoveryRestore { user, password, id }, |_| {}).await;
        set(1.0, "");
        ctx.nav.replace(&[result_page(&ctx, r.map_err(|f| (f.message, f.detail)), tr("Your system was restored. Restart to use it."))]);
    });
}

// ─────────────────────────── Reinstall ───────────────────────────

fn network_page(ctx: &C) -> adw::NavigationPage {
    let content = vbox(18);
    content.append(&heading("network-wireless-symbolic", tr("Connect to the Internet"), tr("MAN Recovery downloads the newest MAN. Connect to a network first.")));
    let status_group = adw::PreferencesGroup::new();
    let status = adw::ActionRow::builder().title(tr("Checking the connection…")).build();
    let status_icon = icon("network-offline-symbolic");
    status.add_prefix(&status_icon);
    status_group.add(&status);
    content.append(&status_group);
    let wifi_group = adw::PreferencesGroup::builder().title(tr("Wi-Fi Networks")).visible(false).build();
    let spinner = adw::Spinner::builder().visible(false).build();
    let refresh = gtk::Button::builder().icon_name("view-refresh-symbolic").tooltip_text(tr("Search Again")).valign(gtk::Align::Center).build();
    refresh.add_css_class("flat");
    let suffix = gtk::Box::builder().spacing(6).build();
    suffix.append(&spinner);
    suffix.append(&refresh);
    wifi_group.set_header_suffix(Some(&suffix));
    let list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).build();
    list.add_css_class("boxed-list");
    wifi_group.add(&list);
    content.append(&wifi_group);
    let next = pill(tr("_Continue"), Some("suggested-action"));
    next.set_sensitive(false);

    let update: Rc<dyn Fn()> = {
        let (status, status_icon, next, wifi_group) = (status.clone(), status_icon.clone(), next.clone(), wifi_group.clone());
        Rc::new(move || {
            let (status, status_icon, next, wifi_group) = (status.clone(), status_icon.clone(), next.clone(), wifi_group.clone());
            glib::spawn_future_local(async move {
                let Ok(st) = gio::spawn_blocking(net_status).await else { return };
                status.set_title(if st.connected { tr("Connected") } else { tr("Not Connected") });
                status.set_subtitle(&glib::markup_escape_text(&st.description));
                status_icon.set_icon_name(Some(if st.connected { "network-transmit-receive-symbolic" } else { "network-offline-symbolic" }));
                wifi_group.set_visible(st.wifi_device);
                next.set_sensitive(st.connected);
            });
        })
    };
    let fill: Rc<RefCell<Box<dyn Fn()>>> = Rc::new(RefCell::new(Box::new(|| {})));
    {
        let (ctx, list, spinner, refresh, update) = (ctx.clone(), list.clone(), spinner.clone(), refresh.clone(), update.clone());
        let fill_self = fill.clone();
        *fill.borrow_mut() = Box::new(move || {
            let (ctx, list, spinner, refresh, update, fill_self) = (ctx.clone(), list.clone(), spinner.clone(), refresh.clone(), update.clone(), fill_self.clone());
            spinner.set_visible(true);
            refresh.set_sensitive(false);
            glib::spawn_future_local(async move {
                let nets = gio::spawn_blocking(wifi_networks).await.unwrap_or_default();
                spinner.set_visible(false);
                refresh.set_sensitive(true);
                list.remove_all();
                if nets.is_empty() {
                    list.append(&adw::ActionRow::builder().title(tr("No networks found")).build());
                }
                for n in nets {
                    let row = adw::ActionRow::builder().title(glib::markup_escape_text(&n.ssid)).activatable(!n.in_use).build();
                    row.add_prefix(&icon(signal_icon(n.signal)));
                    if n.in_use {
                        row.set_subtitle(tr("Connected"));
                        row.add_suffix(&icon("object-select-symbolic"));
                    } else if n.secure {
                        row.add_suffix(&icon("system-lock-screen-symbolic"));
                    }
                    let (ctx, update, fill) = (ctx.clone(), update.clone(), fill_self.clone());
                    row.connect_activated(move |row| {
                        let c2 = ctx.clone();
                        connect_wifi(&ctx.window, Rc::new(move |t: &str| c2.toast(t)), row, &n, update.clone(), fill.clone())
                    });
                    list.append(&row);
                }
            });
        });
    }
    {
        let f = fill.clone();
        refresh.connect_clicked(move |_| (f.borrow())());
    }
    {
        let ctx = ctx.clone();
        next.connect_clicked(move |_| ctx.nav.push(&release_page(&ctx)));
    }
    let page = card_page("network", tr("Internet"), &content, Some(next.upcast_ref()));
    {
        let (u, f) = (update.clone(), fill.clone());
        page.connect_shown(move |_| {
            u();
            (f.borrow())();
        });
    }
    {
        let u = update.clone();
        let weak = page.downgrade();
        let ctx = ctx.clone();
        glib::timeout_add_seconds_local(3, move || {
            let Some(page) = weak.upgrade() else { return glib::ControlFlow::Break };
            if ctx.nav.visible_page().as_ref() == Some(&page) {
                u();
            }
            glib::ControlFlow::Continue
        });
    }
    page
}

/// Looks up the newest MAN, then asks before reinstalling.
fn release_page(ctx: &C) -> adw::NavigationPage {
    let content = vbox(18);
    let head = heading("folder-download-symbolic", tr("Reinstall MAN"), tr("Looking for the newest MAN…"));
    content.append(&head);
    let spinner = adw::Spinner::builder().halign(gtk::Align::Center).height_request(48).build();
    content.append(&spinner);
    let points = vbox(8);
    for t in [
        tr("Your files, accounts and settings stay."),
        tr("Apps you installed as Flatpaks stay; other apps you added may need to be installed again."),
        tr("The system as it is now is kept as a snapshot."),
        tr("Keep the computer plugged in. This takes a while."),
    ] {
        let row = gtk::Box::builder().spacing(10).build();
        row.append(&icon("object-select-symbolic"));
        let l = gtk::Label::builder().label(t).wrap(true).xalign(0.0).hexpand(true).build();
        row.append(&l);
        points.append(&row);
    }
    points.set_visible(false);
    content.append(&points);
    let go = pill(tr("_Reinstall MAN"), Some("suggested-action"));
    go.set_sensitive(false);
    let page = card_page("release", tr("Reinstall MAN"), &content, Some(go.upcast_ref()));
    let found: Rc<RefCell<Option<RecoveryRelease>>> = Rc::default();
    {
        let (ctx, head, spinner, points, go, found) = (ctx.clone(), head.clone(), spinner.clone(), points.clone(), go.clone(), found.clone());
        glib::spawn_future_local(async move {
            let mut rel = None;
            let r = backend::call(&Request::RecoveryFindRelease, |ev| {
                if let Event::Release { release } = ev {
                    rel = Some(release.clone());
                }
            })
            .await;
            spinner.set_visible(false);
            let description = head.last_child().and_downcast::<gtk::Label>();
            match (r, rel) {
                (Ok(_), Some(rel)) => {
                    if let Some(d) = description {
                        d.set_label(&trf("MAN {version} ({size}) will be downloaded and installed on this computer.", &[("version", &rel.version), ("size", &glib::format_size(rel.size))]));
                    }
                    points.set_visible(true);
                    go.set_sensitive(true);
                    *found.borrow_mut() = Some(rel);
                }
                (Err(f), _) => {
                    if let Some(d) = description {
                        d.set_label(&f.message);
                    }
                }
                _ => ctx.toast(tr("No MAN for this computer was found.")),
            }
        });
    }
    {
        let ctx = ctx.clone();
        go.connect_clicked(move |_| {
            let Some(rel) = found.borrow().clone() else { return };
            let (page, set) = working_page(&ctx, "folder-download-symbolic", tr("Reinstalling MAN"));
            ctx.nav.push(&page);
            let ctx = ctx.clone();
            glib::spawn_future_local(async move {
                let (user, password) = ctx.who.borrow().clone();
                let mut title = String::new();
                let r = backend::call(&Request::RecoveryReinstall { user, password, version: rel.version.clone() }, |ev| match ev {
                    Event::Step { title: t, .. } => {
                        title = crate::i18n::tr_dyn(t).to_string();
                        set(0.0, &title);
                    }
                    Event::Progress { fraction, detail } => set(*fraction, &format!("{title}\n{}", crate::i18n::tr_dyn(detail))),
                    _ => {}
                })
                .await;
                ctx.nav.replace(&[result_page(&ctx, r.map_err(|f| (f.message, f.detail)), tr("MAN was installed again. Your files and accounts were kept. Restart to use it."))]);
            });
        });
    }
    page
}

// ─────────────────────────── Working and done ───────────────────────────

/// A page with a progress bar; the returned function sets fraction and text.
fn working_page(ctx: &C, icon_name: &str, title: &str) -> (adw::NavigationPage, Rc<dyn Fn(f64, &str)>) {
    let content = vbox(18);
    content.append(&heading(icon_name, title, tr("Do not turn the computer off.")));
    let bar = gtk::ProgressBar::builder().build();
    bar.add_css_class("install-progress");
    let detail = label("", &["dim-label"]);
    content.append(&bar);
    content.append(&detail);
    let page = card_page("working", title, &content, None);
    page.set_can_pop(false);
    ctx.busy.set(true);
    let pulse = {
        let bar = bar.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(400), move || {
            if bar.fraction() == 0.0 {
                bar.pulse();
            }
            glib::ControlFlow::Continue
        })
    };
    let pulse = Rc::new(RefCell::new(Some(pulse)));
    let set: Rc<dyn Fn(f64, &str)> = Rc::new(move |f: f64, text: &str| {
        if f > 0.0 {
            if let Some(id) = pulse.borrow_mut().take() {
                id.remove();
            }
            bar.set_fraction(f);
        }
        if !text.is_empty() {
            detail.set_label(text);
        }
    });
    (page, set)
}

fn result_page(ctx: &C, r: Result<String, (String, String)>, done: &str) -> adw::NavigationPage {
    ctx.busy.set(false);
    let content = vbox(18);
    let ok = r.is_ok();
    let img = gtk::Image::from_icon_name(if ok { "object-select-symbolic" } else { "dialog-error-symbolic" });
    img.add_css_class(if ok { "recovery-done" } else { "recovery-failed" });
    content.append(&img);
    match &r {
        Ok(msg) => {
            content.append(&label(tr("Done"), &["recovery-title"]));
            let _ = msg;
            content.append(&label(done, &["dim-label"]));
        }
        Err((msg, detail)) => {
            content.append(&label(tr("That Did Not Work"), &["recovery-title"]));
            content.append(&label(msg, &["dim-label"]));
            let more = gtk::Button::builder().label(tr("_Details")).use_underline(true).halign(gtk::Align::Center).build();
            more.add_css_class("flat");
            let (w, d) = (ctx.window.clone(), detail.clone());
            more.connect_clicked(move |_| details_dialog(&w, tr("Details"), &d));
            content.append(&more);
        }
    }
    let buttons = vbox(8);
    let primary = pill(tr("_Restart"), Some("suggested-action"));
    primary.connect_clicked(|_| restart());
    buttons.append(&primary);
    if !ok {
        let back = pill(tr("_Back to MAN Recovery"), None);
        let ctx = ctx.clone();
        back.connect_clicked(move |_| ctx.nav.replace(&[options_page(&ctx)]));
        buttons.append(&back);
    }
    let page = card_page("result", tr("MAN Recovery"), &content, Some(buttons.upcast_ref()));
    page.set_can_pop(false);
    page
}

// ─────────────────────────── Window ───────────────────────────

pub fn run(app: &adw::Application) {
    crate::i18n::set_language(crate::langui::system_language().locale);
    adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceDark);
    load_css();
    let provider = gtk::CssProvider::new();
    provider.load_from_string(CSS);
    let display = gdk::Display::default().expect("display");
    #[allow(deprecated)]
    gtk::style_context_add_provider_for_display(&display, &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

    let window = adw::ApplicationWindow::builder().application(app).title(tr("MAN Recovery")).default_width(1280).default_height(800).deletable(false).build();
    let root = gtk::Overlay::new();
    root.add_css_class("recovery-backdrop");
    root.set_child(Some(&gtk::Box::builder().hexpand(true).vexpand(true).build()));
    let column = vbox(0);
    column.set_halign(gtk::Align::Center);
    column.set_valign(gtk::Align::Center);
    let card = gtk::Box::builder().orientation(gtk::Orientation::Vertical).width_request(720).height_request(620).margin_top(24).build();
    card.add_css_class("recovery-card");
    card.set_overflow(gtk::Overflow::Hidden);
    let nav = adw::NavigationView::new();
    nav.set_vexpand(true);
    let toasts = adw::ToastOverlay::new();
    toasts.set_child(Some(&nav));
    card.append(&toasts);
    column.append(&card);
    // Under the card: always a way out.
    let footer = gtk::Box::builder().spacing(12).halign(gtk::Align::Center).margin_top(18).margin_bottom(24).build();
    footer.add_css_class("recovery-footer");
    let restart_b = gtk::Button::builder().label(tr("_Restart")).use_underline(true).build();
    let off_b = gtk::Button::builder().label(tr("_Shut Down")).use_underline(true).build();
    for b in [&restart_b, &off_b] {
        b.add_css_class("flat");
        footer.append(b);
    }
    column.append(&footer);
    root.add_overlay(&column);
    window.set_content(Some(&root));

    let ctx: C = Rc::new(Ctx { window: window.clone(), nav: nav.clone(), toasts, info: RefCell::default(), who: RefCell::default(), busy: Cell::new(false) });
    {
        let ctx = ctx.clone();
        restart_b.connect_clicked(move |_| {
            if !ctx.busy.get() {
                restart();
            }
        });
    }
    {
        let ctx = ctx.clone();
        off_b.connect_clicked(move |_| {
            if !ctx.busy.get() {
                power_off();
            }
        });
    }
    window.connect_close_request(|_| glib::Propagation::Stop);

    // A first page while the system helper looks at the disk.
    let wait = vbox(18);
    wait.append(&heading("system-reboot-symbolic", tr("MAN Recovery"), tr("Looking at this computer…")));
    wait.append(&adw::Spinner::builder().halign(gtk::Align::Center).height_request(48).build());
    nav.add(&card_page("wait", tr("MAN Recovery"), &wait, None));
    {
        let ctx = ctx.clone();
        glib::spawn_future_local(async move {
            let mut found = None;
            let r = backend::call(&Request::RecoveryInfo, |ev| {
                if let Event::Recovery { info } = ev {
                    found = Some(info.clone());
                }
            })
            .await;
            match (r, found) {
                (Ok(_), Some(info)) => {
                    *ctx.info.borrow_mut() = info;
                    ctx.nav.replace(&[unlock_page(&ctx)]);
                }
                (r, _) => {
                    let (m, d) = r.err().map(|f| (f.message, f.detail)).unwrap_or_default();
                    ctx.nav.replace(&[result_page(&ctx, Err((m, d)), "")]);
                }
            }
        });
    }
    window.fullscreen();
    window.present();
}

pub fn main() -> glib::ExitCode {
    crate::i18n::init();
    let app = adw::Application::builder().application_id("com.man.Recovery").build();
    app.connect_activate(|app| {
        if let Some(w) = app.active_window() {
            w.present();
        } else {
            run(app);
        }
    });
    app.run()
}
