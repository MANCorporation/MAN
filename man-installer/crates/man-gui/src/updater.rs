//! "MAN Update": installs a signed `.upd` system update, and the notice
//! shown after logging in to an updated system (`man-update --notify`).
use crate::i18n::{tr, trf, tr_dyn};
use crate::backend;
use crate::ui::*;
use adw::prelude::*;
use gtk::{gio, glib};
use man_common::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Weight of each backend step in the overall progress bar.
const WEIGHTS: [f64; 6] = [2.0, 3.0, 2.0, 70.0, 8.0, 15.0];

struct Ctx {
    window: adw::ApplicationWindow,
    nav: adw::NavigationView,
    path: String,
    info: RefCell<Option<UpdateInfo>>,
    target: RefCell<Option<String>>,
    accepted: Cell<bool>,
    busy: Cell<bool>,
}

type C = Rc<Ctx>;

impl Ctx {
    fn info(&self) -> UpdateInfo {
        self.info.borrow().clone().unwrap_or_default()
    }
}

fn info_row(title: &str, value: &str, icon_name: &str) -> adw::ActionRow {
    let row = adw::ActionRow::builder().title(title).subtitle(glib::markup_escape_text(value)).subtitle_selectable(true).build();
    row.add_css_class("property");
    row.add_prefix(&icon(icon_name));
    row
}

fn bottom_bar(button: &gtk::Button, hint: &gtk::Label) -> gtk::Box {
    let b = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(6).build();
    b.append(hint);
    b.append(button);
    b
}

fn status_page_with_header(title: &str, tag: &str, status: &adw::StatusPage, can_pop: bool) -> adw::NavigationPage {
    let tv = adw::ToolbarView::new();
    tv.add_top_bar(&adw::HeaderBar::builder().show_title(false).build());
    tv.set_content(Some(status));
    adw::NavigationPage::builder().title(title).tag(tag).child(&tv).can_pop(can_pop).build()
}

// ─────────────────────────── Loading / errors ───────────────────────────

fn loading_page(ctx: &C) -> adw::NavigationPage {
    let status = adw::StatusPage::builder().title(tr("Checking the Update")).description(tr("Making sure the update is genuine and complete…")).vexpand(true).build();
    let spinner = adw::Spinner::builder().width_request(48).height_request(48).halign(gtk::Align::Center).build();
    status.set_child(Some(&spinner));
    let page = status_page_with_header(tr("MAN Update"), "loading", &status, false);
    let ctx = ctx.clone();
    glib::spawn_future_local(async move {
        let mut info = None;
        let res = backend::call_unprivileged(&Request::UpdateInspect { path: ctx.path.clone() }, |ev| {
            if let Event::Update { info: i } = ev {
                info = Some(i.clone());
            }
        })
        .await;
        match (res, info) {
            (Ok(_), Some(i)) => {
                *ctx.info.borrow_mut() = Some(i);
                ctx.nav.replace(&[overview_page(&ctx)]);
            }
            (Err(f), _) => ctx.nav.replace(&[problem_page(tr("This Update Cannot Be Installed"), &f.message, "dialog-error-symbolic")]),
            (Ok(_), None) => ctx.nav.replace(&[problem_page(tr("This Update Cannot Be Installed"), tr("The update could not be read."), "dialog-error-symbolic")]),
        }
    });
    page
}

fn problem_page(title: &str, message: &str, icon_name: &str) -> adw::NavigationPage {
    let status = adw::StatusPage::builder().icon_name(icon_name).title(title).description(glib::markup_escape_text(message)).vexpand(true).build();
    status_page_with_header(title, "problem", &status, false)
}

// ─────────────────────────── Overview ───────────────────────────

fn overview_page(ctx: &C) -> adw::NavigationPage {
    let info = ctx.info();
    let m = &info.manifest;
    let newer = version_cmp(&m.version, &info.current_version) == std::cmp::Ordering::Greater;
    let prefs = adw::PreferencesPage::new();

    let head = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(6).margin_top(12).build();
    let hero = gtk::Image::from_icon_name("com.man.Updater");
    hero.set_pixel_size(128);
    head.append(&hero);
    head.append(&label(&format!("{} {}", m.name, m.version), &["title-1"]));
    head.append(&label(
        &if newer { trf("Update from {name} {version}", &[("name", &m.name), ("version", &info.current_version)]) } else { trf("{name} {version} is installed", &[("name", &m.name), ("version", &info.current_version)]) },
        &["dim-label"],
    ));
    let hg = adw::PreferencesGroup::new();
    hg.add(&head);
    prefs.add(&hg);

    let details = adw::PreferencesGroup::builder().title(tr("Details")).build();
    details.add(&info_row(tr("Version"), &m.version, "software-update-available-symbolic"));
    if !m.date.is_empty() {
        details.add(&info_row(tr("Released"), &m.date, "x-office-calendar-symbolic"));
    }
    details.add(&info_row(tr("Download Size"), &human_size(m.payload_size), "folder-download-symbolic"));
    details.add(&info_row(tr("Space Needed"), &human_size(m.installed_size), "drive-harddisk-symbolic"));
    let verified = info_row(tr("Authenticity"), &trf("Verified, signed by “{signer}”", &[("signer", &info.signed_by)]), "security-high-symbolic");
    details.add(&verified);
    prefs.add(&details);

    if !info.notes.trim().is_empty() {
        let notes = adw::PreferencesGroup::builder().title(tr("What's New")).build();
        let text = gtk::Label::builder().label(info.notes.trim()).wrap(true).xalign(0.0).selectable(true).margin_top(12).margin_bottom(12).margin_start(12).margin_end(12).build();
        let card = gtk::Box::new(gtk::Orientation::Vertical, 0);
        card.add_css_class("card");
        card.append(&text);
        notes.add(&card);
        prefs.add(&notes);
    }

    let next = pill(tr("_Continue"), Some("suggested-action"));
    let hint = label("", &["dim-label", "caption"]);
    hint.set_visible(!newer);
    next.set_sensitive(newer);
    if !newer {
        hint.set_label(&trf("This update is not newer than the installed {name} {version}.", &[("name", &m.name), ("version", &info.current_version)]));
    }
    {
        let ctx = ctx.clone();
        next.connect_clicked(move |_| {
            let p = if ctx.info().license.trim().is_empty() { target_page(&ctx) } else { license_page(&ctx) };
            ctx.nav.push(&p);
        });
    }
    page(tr("Update"), "overview", &prefs, Some(bottom_bar(&next, &hint).upcast_ref()))
}

// ─────────────────────────── License ───────────────────────────

fn license_page(ctx: &C) -> adw::NavigationPage {
    let content = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(12).margin_top(12).margin_bottom(12).margin_start(12).margin_end(12).build();
    content.append(&label(tr("Please read the license terms of this update."), &["dim-label"]));
    let view = gtk::TextView::builder().editable(false).cursor_visible(false).wrap_mode(gtk::WrapMode::WordChar).top_margin(12).bottom_margin(12).left_margin(12).right_margin(12).build();
    view.buffer().set_text(ctx.info().license.trim());
    let scroll = gtk::ScrolledWindow::builder().child(&view).vexpand(true).min_content_height(240).build();
    scroll.add_css_class("card");
    scroll.set_overflow(gtk::Overflow::Hidden);
    content.append(&scroll);
    let accept = gtk::CheckButton::builder().label(tr("I accept the license terms")).halign(gtk::Align::Center).active(ctx.accepted.get()).build();
    content.append(&accept);
    let clamp = adw::Clamp::builder().maximum_size(720).child(&content).build();

    let next = pill(tr("_Next"), Some("suggested-action"));
    next.set_sensitive(ctx.accepted.get());
    let hint = label("", &["caption"]);
    hint.set_visible(false);
    {
        let (ctx, next) = (ctx.clone(), next.clone());
        accept.connect_toggled(move |c| {
            ctx.accepted.set(c.is_active());
            next.set_sensitive(c.is_active());
        });
    }
    {
        let ctx = ctx.clone();
        next.connect_clicked(move |_| ctx.nav.push(&target_page(&ctx)));
    }
    page(tr("License Agreement"), "license", &clamp, Some(bottom_bar(&next, &hint).upcast_ref()))
}

// ─────────────────────────── Target ───────────────────────────

fn target_page(ctx: &C) -> adw::NavigationPage {
    let info = ctx.info();
    let prefs = adw::PreferencesPage::new();
    let g = adw::PreferencesGroup::builder()
        .title(tr("Where Should the Update Be Installed?"))
        .description(tr("The system is replaced; files in your Home folder are not touched."))
        .build();
    let list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).build();
    list.add_css_class("boxed-list");
    let anchor = gtk::CheckButton::new();
    let next = pill(tr("_Next"), Some("suggested-action"));
    let hint = label(tr("Choose an installation."), &["dim-label", "caption"]);
    let mut first_ok = None;
    for t in &info.targets {
        let check = gtk::CheckButton::new();
        check.set_group(Some(&anchor));
        let mut sub = vec![format!("{} ({})", t.model, t.disk)];
        if t.running {
            sub.push(tr("running now").into());
            sub.push(trf("{size} free", &[("size", &human_size(t.free))]));
        }
        if t.encrypted {
            sub.push("encrypted".into());
        }
        let mut subtitle = sub.join("  ·  ");
        if let Some(p) = &t.problem {
            subtitle = format!("{subtitle}\n{p}");
        }
        let title = if t.running { trf("{name} {version} on this computer", &[("name", &info.manifest.name), ("version", &info.current_version)]) } else { trf("{name} on {model}", &[("name", &info.manifest.name), ("model", &t.model)]) };
        let row = adw::ActionRow::builder().title(glib::markup_escape_text(&title)).subtitle(glib::markup_escape_text(&subtitle)).activatable_widget(&check).build();
        row.add_prefix(&check);
        row.add_suffix(&icon(if t.encrypted { "channel-secure-symbolic" } else { "computer-symbolic" }));
        row.set_sensitive(t.problem.is_none());
        if t.problem.is_none() && first_ok.is_none() {
            first_ok = Some((check.clone(), t.device.clone()));
        }
        let (ctx2, dev, next2, hint2) = (ctx.clone(), t.device.clone(), next.clone(), hint.clone());
        check.connect_toggled(move |c| {
            if c.is_active() {
                *ctx2.target.borrow_mut() = Some(dev.clone());
                next2.set_sensitive(true);
                hint2.set_visible(false);
            }
        });
        list.append(&row);
    }
    if info.targets.is_empty() {
        list.append(&adw::ActionRow::builder().title(tr("No MAN installation found")).subtitle(tr("Updates are installed from a running, installed MAN system, not from the live session.")).build());
    }
    g.add(&list);
    prefs.add(&g);
    next.set_sensitive(false);
    if let Some((c, _)) = first_ok {
        c.set_active(true);
    }
    {
        let ctx = ctx.clone();
        next.connect_clicked(move |_| ctx.nav.push(&confirm_page(&ctx)));
    }
    page(tr("Install Location"), "target", &prefs, Some(bottom_bar(&next, &hint).upcast_ref()))
}

// ─────────────────────────── Confirm ───────────────────────────

fn confirm_page(ctx: &C) -> adw::NavigationPage {
    let info = ctx.info();
    let m = &info.manifest;
    let target = ctx.target.borrow().clone().unwrap_or_default();
    let t = info.targets.iter().find(|t| t.device == target).cloned().unwrap_or_default();
    let prefs = adw::PreferencesPage::new();

    let summary = adw::PreferencesGroup::builder().title(tr("Summary")).build();
    summary.add(&info_row(tr("Update"), &format!("{} {} → {} {}", m.name, info.current_version, m.name, m.version), "software-update-available-symbolic"));
    summary.add(&info_row(tr("Installation"), &format!("{} ({})", t.model, t.disk), "drive-harddisk-symbolic"));
    prefs.add(&summary);

    let kept = adw::PreferencesGroup::builder().title(tr("Kept")).build();
    for (title, sub, ic) in [
        (tr("Your Files"), tr("Everything in your Home folder"), "user-home-symbolic"),
        (tr("Accounts and Passwords"), tr("Users, groups, the computer name"), "system-users-symbolic"),
        (tr("Connections and Settings"), tr("Wi-Fi, time zone, keyboard, look and snapshots"), "emblem-system-symbolic"),
        (tr("Flatpak Apps"), tr("Plus anything in /usr/local and /opt"), "application-x-addon-symbolic"),
    ] {
        let row = adw::ActionRow::builder().title(title).subtitle(sub).build();
        row.add_prefix(&icon(ic));
        row.add_suffix(&icon("object-select-symbolic"));
        kept.add(&row);
    }
    prefs.add(&kept);

    let replaced = adw::PreferencesGroup::builder()
        .title(tr("Replaced"))
        .description(tr("Software installed with apt and other changes to system files are replaced by the new system."))
        .build();
    let row = adw::ActionRow::builder().title(tr("The Operating System")).subtitle(trf("Replaced by {name} {version}. The current system can be restored from a snapshot.", &[("name", &m.name), ("version", &m.version)])).build();
    row.add_prefix(&icon("view-refresh-symbolic"));
    replaced.add(&row);
    prefs.add(&replaced);

    let restart = adw::PreferencesGroup::new();
    let r = adw::ActionRow::builder()
        .title(tr("The computer restarts to install the update"))
        .subtitle(tr("After preparing, MAN restarts, installs the update in a few seconds and restarts again."))
        .build();
    r.add_prefix(&icon("dialog-information-symbolic"));
    restart.add(&r);
    prefs.add(&restart);

    let go = pill(tr("_Install Update"), Some("suggested-action"));
    {
        let ctx = ctx.clone();
        go.connect_clicked(move |_| ask_snapshot(&ctx));
    }
    page(tr("Ready to Install"), "confirm", &prefs, Some(go.upcast_ref()))
}

fn ask_snapshot(ctx: &C) {
    let dialog = adw::AlertDialog::new(
        Some(tr("Create a Snapshot First?")),
        Some(tr("A snapshot saves the current system, so you can go back to it if something does not work after the update. This is recommended.")),
    );
    dialog.add_response("cancel", tr("_Cancel"));
    dialog.add_response("skip", tr("_Don't Create"));
    dialog.add_response("snapshot", tr("_Create Snapshot"));
    dialog.set_response_appearance("snapshot", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("snapshot"));
    dialog.set_close_response("cancel");
    let ctx = ctx.clone();
    glib::spawn_future_local(async move {
        match dialog.choose_future(Some(&ctx.window)).await.as_str() {
            "snapshot" => start(&ctx, true),
            "skip" => start(&ctx, false),
            _ => {}
        }
    });
}

// ─────────────────────────── Progress ───────────────────────────

fn start(ctx: &C, snapshot: bool) {
    let content = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(18).valign(gtk::Align::Center).margin_start(24).margin_end(24).build();
    let hero = gtk::Image::from_icon_name("com.man.Updater");
    hero.set_pixel_size(128);
    let title = label(&trf("Preparing {name} {version}", &[("name", &ctx.info().manifest.name), ("version", &ctx.info().manifest.version)]), &["title-1"]);
    let step_label = label(tr("Starting…"), &["dim-label"]);
    let bar = gtk::ProgressBar::new();
    bar.add_css_class("install-progress");
    let detail = label("", &["caption", "dim-label"]);
    let log_view = gtk::TextView::builder().editable(false).cursor_visible(false).wrap_mode(gtk::WrapMode::WordChar).build();
    log_view.add_css_class("log-view");
    log_view.add_css_class("card");
    let log_scroll = gtk::ScrolledWindow::builder().min_content_height(180).child(&log_view).build();
    let expander = gtk::Expander::builder().label(tr("Details")).child(&log_scroll).build();
    for w in [hero.upcast_ref::<gtk::Widget>(), title.upcast_ref(), step_label.upcast_ref(), bar.upcast_ref(), detail.upcast_ref(), expander.upcast_ref()] {
        content.append(w);
    }
    let clamp = adw::Clamp::builder().maximum_size(560).child(&content).vexpand(true).build();
    let progress_page = page(tr("Preparing"), "progress", &clamp, None);
    progress_page.set_can_pop(false);
    ctx.nav.push(&progress_page);
    ctx.busy.set(true);

    let ctx = ctx.clone();
    let req = Request::UpdateStage { path: ctx.path.clone(), device: ctx.target.borrow().clone().unwrap_or_default(), snapshot };
    glib::spawn_future_local(async move {
        let buffer = log_view.buffer();
        let mut current = 0usize;
        let total: f64 = WEIGHTS.iter().sum();
        let res = backend::call(&req, |ev| match ev {
            Event::Step { index, total: n, title } => {
                current = (*index as usize).saturating_sub(1).min(WEIGHTS.len() - 1);
                step_label.set_label(&trf("Step {index} of {n}  ·  {title}", &[("index", &index), ("n", &n), ("title", &tr_dyn(title))]));
                detail.set_label("");
                bar.set_fraction(WEIGHTS[..current].iter().sum::<f64>() / total);
            }
            Event::Progress { fraction, detail: d } => {
                bar.set_fraction((WEIGHTS[..current].iter().sum::<f64>() + WEIGHTS[current] * fraction) / total);
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
        let log = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string();
        match res {
            Ok(message) => ctx.nav.replace(&[ready_page(&ctx, &message)]),
            Err(f) => {
                let p = failed_page(&ctx, &f.message, &format!("{}\n\n── {} ──\n{}", f.detail, tr("Log"), log));
                ctx.nav.push(&p);
            }
        }
    });
}

fn ready_page(ctx: &C, message: &str) -> adw::NavigationPage {
    let status = adw::StatusPage::builder()
        .icon_name("system-reboot-symbolic")
        .title(tr("Restart to Install"))
        .description(format!("{}\n{}", tr_dyn(message), tr("The computer restarts twice. Your files are safe.")))
        .vexpand(true)
        .build();
    let buttons = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(12).halign(gtk::Align::Center).build();
    let now = pill(tr("_Restart Now"), Some("suggested-action"));
    now.set_width_request(240);
    let later = pill(tr("Restart _Later"), None);
    later.set_width_request(240);
    let countdown = label("", &["dim-label", "caption"]);
    buttons.append(&now);
    buttons.append(&later);
    buttons.append(&countdown);
    status.set_child(Some(&buttons));
    now.connect_clicked(|_| spawn_detached(&["systemctl", "reboot"]));

    // Restart automatically after a short countdown, unless postponed.
    let remaining = Rc::new(Cell::new(30u32));
    let source: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));
    {
        let (remaining, countdown, s2) = (remaining.clone(), countdown.clone(), source.clone());
        let id = glib::timeout_add_seconds_local(1, move || {
            let r = remaining.get().saturating_sub(1);
            remaining.set(r);
            countdown.set_label(&trf("Restarting automatically in {r} seconds", &[("r", &r)]));
            if r == 0 {
                s2.borrow_mut().take();
                spawn_detached(&["systemctl", "reboot"]);
                return glib::ControlFlow::Break;
            }
            glib::ControlFlow::Continue
        });
        *source.borrow_mut() = Some(id);
    }
    countdown.set_label(tr("Restarting automatically in 30 seconds"));
    {
        let (ctx, source, countdown) = (ctx.clone(), source.clone(), countdown.clone());
        later.connect_clicked(move |_| {
            if let Some(id) = source.borrow_mut().take() {
                id.remove();
            }
            countdown.set_label(tr("The update is installed the next time you restart."));
            ctx.window.close();
        });
    }
    let p = status_page_with_header(tr("Ready"), "ready", &status, false);
    let now2 = now.clone();
    p.connect_shown(move |_| {
        now2.grab_focus();
    });
    p
}

fn failed_page(ctx: &C, message: &str, details: &str) -> adw::NavigationPage {
    let status = adw::StatusPage::builder()
        .icon_name("dialog-error-symbolic")
        .title(tr("The Update Could Not Be Prepared"))
        .description(format!("{}\n\n{}", glib::markup_escape_text(&tr_dyn(message)), tr("Nothing was changed on your system.")))
        .vexpand(true)
        .build();
    let buttons = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(12).halign(gtk::Align::Center).build();
    let back = pill(tr("_Try Again"), Some("suggested-action"));
    back.set_width_request(240);
    let show = pill(tr("_Show Details"), None);
    show.set_width_request(240);
    buttons.append(&back);
    buttons.append(&show);
    status.set_child(Some(&buttons));
    {
        let ctx = ctx.clone();
        back.connect_clicked(move |_| {
            ctx.nav.pop_to_tag("confirm");
        });
    }
    {
        let (ctx, details) = (ctx.clone(), details.to_string());
        show.connect_clicked(move |_| details_dialog(&ctx.window, tr("Update Log"), &details));
    }
    status_page_with_header(tr("Failed"), "failed", &status, false)
}

pub fn run(app: &adw::Application, path: String) {
    load_css();
    let window = adw::ApplicationWindow::builder().application(app).title(tr("MAN Update")).default_width(760).default_height(680).build();
    let nav = adw::NavigationView::new();
    window.set_content(Some(&nav));
    let ctx: C = Rc::new(Ctx { window: window.clone(), nav, path, info: RefCell::new(None), target: RefCell::new(None), accepted: Cell::new(false), busy: Cell::new(false) });
    {
        let ctx = ctx.clone();
        window.connect_close_request(move |_| if ctx.busy.get() { glib::Propagation::Stop } else { glib::Propagation::Proceed });
    }
    ctx.nav.add(&loading_page(&ctx));
    window.present();
}

// ─────────────────────────── After an update ───────────────────────────

fn seen_file() -> std::path::PathBuf {
    glib::user_data_dir().join("man/update-notified")
}

/// After login: tell the user once per update that the system was updated.
pub fn notify(app: &adw::Application) {
    let Some(last) = std::fs::read_to_string(LAST_UPDATE).ok().and_then(|t| serde_json::from_str::<LastUpdate>(&t).ok()) else {
        return;
    };
    if std::fs::read_to_string(seen_file()).map(|s| s.trim() == last.version).unwrap_or(false) {
        return;
    }
    load_css();
    let window = adw::ApplicationWindow::builder().application(app).title(tr("MAN Update")).default_width(460).default_height(500).resizable(false).build();
    let status = adw::StatusPage::builder()
        .icon_name("com.man.Updater")
        .title(tr("Your System Was Updated"))
        .description(trf("MAN was updated to version {version}.\nPreviously: MAN {previous}.", &[("version", &last.version), ("previous", &last.previous)]))
        .vexpand(true)
        .build();
    let done = pill(tr("_Done"), Some("suggested-action"));
    done.set_width_request(200);
    status.set_child(Some(&done));
    let tv = adw::ToolbarView::new();
    tv.add_top_bar(&adw::HeaderBar::builder().show_title(false).build());
    tv.set_content(Some(&status));
    window.set_content(Some(&tv));
    let version = last.version.clone();
    let mark_seen = move || {
        let f = seen_file();
        let _ = std::fs::create_dir_all(f.parent().unwrap());
        let _ = std::fs::write(&f, format!("{version}\n"));
    };
    {
        let (w, mark) = (window.clone(), mark_seen.clone());
        done.connect_clicked(move |_| {
            mark();
            w.close();
        });
    }
    window.connect_close_request(move |_| {
        mark_seen();
        glib::Propagation::Proceed
    });
    window.present();
    done.grab_focus();
}

pub fn main() -> glib::ExitCode {
    crate::i18n::init();
    let notify_mode = std::env::args().nth(1).as_deref() == Some("--notify");
    let app = adw::Application::builder()
        .application_id(if notify_mode { "com.man.UpdateNotice" } else { "com.man.Updater" })
        .flags(gio::ApplicationFlags::HANDLES_OPEN | gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_open(|app, files, _| {
        if let Some(path) = files.first().and_then(|f| f.path()) {
            run(app, path.to_string_lossy().into_owned());
        }
    });
    app.connect_activate(move |app| {
        if notify_mode {
            notify(app);
        } else if app.active_window().is_none() {
            // Opened without a file: let the user pick one.
            let dialog = gtk::FileDialog::builder().title(tr("Open a MAN Update")).build();
            let filter = gtk::FileFilter::new();
            filter.set_name(Some(tr("MAN updates")));
            filter.add_pattern("*.upd");
            let filters = gio::ListStore::new::<gtk::FileFilter>();
            filters.append(&filter);
            dialog.set_filters(Some(&filters));
            let app = app.clone();
            let hold = app.hold();
            dialog.open(gtk::Window::NONE, gio::Cancellable::NONE, move |res| {
                drop(hold);
                if let Some(path) = res.ok().and_then(|f| f.path()) {
                    run(&app, path.to_string_lossy().into_owned());
                }
            });
        }
    });
    let args: Vec<String> = std::env::args().filter(|a| a != "--notify").collect();
    app.run_with_args(&args)
}
