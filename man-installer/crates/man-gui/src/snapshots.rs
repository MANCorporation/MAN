//! "Snapshots": manage restore points of an installed MAN system.
use crate::i18n::{tr, trf};
use crate::backend;
use crate::ui::*;
use adw::prelude::*;
use gtk::{gio, glib};
use man_common::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Ctx {
    window: adw::ApplicationWindow,
    toasts: adw::ToastOverlay,
    stack: gtk::Stack,
    list: gtk::ListBox,
    spinner: adw::Spinner,
    add: gtk::Button,
    banner: adw::Banner,
    groups: Vec<gtk::Widget>,
    usage: adw::ActionRow,
    daily: adw::SwitchRow,
    keep_daily: adw::SpinRow,
    pre_update: adw::SwitchRow,
    keep_pre_update: adw::SpinRow,
    loading_config: Cell<bool>,
    config_timer: RefCell<Option<glib::SourceId>>,
    busy: Cell<bool>,
}

type C = Rc<Ctx>;

#[derive(PartialEq, Clone, Copy)]
enum Layout {
    Ready,
    /// A restore replaced @system; the running system is the old one.
    RestorePending,
    Unavailable,
}

/// Snapshots are managed when / is @system and /.snapshots is @snapshots.
fn layout() -> Layout {
    let info = std::fs::read_to_string("/proc/self/mountinfo").unwrap_or_default();
    let root_of = |mp: &str| {
        info.lines().find_map(|l| {
            let f: Vec<&str> = l.split(' ').collect();
            (f.len() > 4 && f[4] == mp).then(|| f[3].to_string())
        })
    };
    if root_of(SNAPSHOT_DIR).as_deref() != Some("/@snapshots") {
        return Layout::Unavailable;
    }
    match root_of("/").as_deref() {
        Some("/@system") => Layout::Ready,
        Some(r) if r.starts_with("/@system.old-") => Layout::RestorePending,
        _ => Layout::Unavailable,
    }
}

fn read_snapshots() -> Vec<SnapshotInfo> {
    let mut v: Vec<SnapshotInfo> = std::fs::read_dir(SNAPSHOT_DIR)
        .map(|r| {
            r.filter_map(|e| e.ok())
                .filter(|e| e.path().join("snapshot").is_dir())
                .filter_map(|e| std::fs::read_to_string(e.path().join("info.json")).ok())
                .filter_map(|t| serde_json::from_str(&t).ok())
                .collect()
        })
        .unwrap_or_default();
    v.sort_by(|a, b| b.id.cmp(&a.id));
    v
}

fn format_date(rfc: &str) -> String {
    glib::DateTime::from_iso8601(rfc, None)
        .and_then(|d| d.to_local())
        .and_then(|d| d.format("%e %B %Y, %H:%M"))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| rfc.to_string())
}

fn kind_label(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "install" => (tr("Installation"), "computer-symbolic"),
        "daily" => (tr("Daily"), "alarm-symbolic"),
        "pre-update" => (tr("Before update"), "software-update-available-symbolic"),
        "pre-rollback" => (tr("Before restore"), "edit-undo-symbolic"),
        "pre-reinstall" => (tr("Before reinstall"), "view-refresh-symbolic"),
        _ => (tr("Manual"), "camera-photo-symbolic"),
    }
}

impl Ctx {
    fn toast(&self, t: &str) {
        self.toasts.add_toast(adw::Toast::new(t));
    }

    fn set_busy(&self, busy: bool) {
        self.busy.set(busy);
        self.spinner.set_visible(busy);
        let usable = !busy && layout() == Layout::Ready;
        self.add.set_sensitive(usable);
        self.groups.iter().for_each(|g| g.set_sensitive(usable));
    }

    fn refresh(self: &Rc<Self>) {
        let state = layout();
        if state == Layout::Unavailable {
            self.stack.set_visible_child_name("unavailable");
            self.add.set_visible(false);
            self.banner.set_revealed(false);
            return;
        }
        self.stack.set_visible_child_name("list");
        // After a restore, the running system is about to be replaced: show
        // what happens next and allow nothing else until the restart.
        self.banner.set_revealed(state == Layout::RestorePending);
        self.add.set_sensitive(state == Layout::Ready && !self.busy.get());
        let usable = state == Layout::Ready && !self.busy.get();
        self.groups.iter().for_each(|g| g.set_sensitive(usable));
        self.list.remove_all();
        let snaps = read_snapshots();
        if snaps.is_empty() {
            let row = adw::ActionRow::builder().title(tr("No snapshots yet")).subtitle(tr("Create one before making big changes to the system.")).build();
            self.list.append(&row);
        }
        for s in snaps {
            let (kind, icon_name) = kind_label(&s.kind);
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(&s.description))
                .subtitle(format!("{}  ·  {}", format_date(&s.created), kind))
                .build();
            row.add_prefix(&icon(icon_name));
            let restore = gtk::Button::builder().label(tr("Restore…")).valign(gtk::Align::Center).build();
            restore.add_css_class("flat");
            let delete = gtk::Button::builder().icon_name("user-trash-symbolic").tooltip_text(tr("Delete Snapshot")).valign(gtk::Align::Center).build();
            delete.add_css_class("flat");
            row.add_suffix(&restore);
            row.add_suffix(&delete);
            {
                let ctx = self.clone();
                let s = s.clone();
                restore.connect_clicked(move |_| ask_restore(&ctx, &s));
            }
            {
                let ctx = self.clone();
                let s = s.clone();
                delete.connect_clicked(move |_| ask_delete(&ctx, &s));
            }
            self.list.append(&row);
        }
        self.update_usage();
        self.load_config();
    }

    fn update_usage(&self) {
        let file = gio::File::for_path("/");
        if let Ok(info) = file.query_filesystem_info("filesystem::size,filesystem::free", gio::Cancellable::NONE) {
            let size = info.attribute_uint64("filesystem::size");
            let free = info.attribute_uint64("filesystem::free");
            self.usage.set_subtitle(&trf(
                "{free} free of {size}. System, home and snapshots share this space; snapshots only use space for what changed since they were taken.",
                &[("free", &human_size(free)), ("size", &human_size(size))],
            ));
        }
    }

    fn load_config(&self) {
        let c = std::fs::read_to_string(SNAPSHOT_CONFIG).map(|t| SnapshotConfig::parse(&t)).unwrap_or_default();
        self.loading_config.set(true);
        self.daily.set_active(c.daily);
        self.keep_daily.set_value(c.keep_daily as f64);
        self.pre_update.set_active(c.pre_update);
        self.keep_pre_update.set_value(c.keep_pre_update as f64);
        self.keep_daily.set_sensitive(c.daily);
        self.keep_pre_update.set_sensitive(c.pre_update);
        self.loading_config.set(false);
    }

    /// Settings are saved shortly after the last change, in one request.
    fn schedule_config_save(self: &Rc<Self>) {
        if self.loading_config.get() {
            return;
        }
        self.keep_daily.set_sensitive(self.daily.is_active());
        self.keep_pre_update.set_sensitive(self.pre_update.is_active());
        if let Some(id) = self.config_timer.borrow_mut().take() {
            id.remove();
        }
        let ctx = self.clone();
        let id = glib::timeout_add_local_once(std::time::Duration::from_millis(700), move || {
            ctx.config_timer.borrow_mut().take();
            let config = SnapshotConfig {
                daily: ctx.daily.is_active(),
                keep_daily: ctx.keep_daily.value() as u32,
                pre_update: ctx.pre_update.is_active(),
                keep_pre_update: ctx.keep_pre_update.value() as u32,
            };
            run_request(&ctx, Request::SetSnapshotConfig { config }, false);
        });
        *self.config_timer.borrow_mut() = Some(id);
    }
}

fn run_request(ctx: &C, req: Request, after_rollback: bool) {
    let ctx = ctx.clone();
    ctx.set_busy(true);
    glib::spawn_future_local(async move {
        let res = backend::call(&req, |_| {}).await;
        ctx.set_busy(false);
        match res {
            Ok(msg) => {
                if after_rollback {
                    ask_restart(&ctx, &msg);
                } else {
                    ctx.toast(&msg);
                }
            }
            Err(f) => {
                let dialog = adw::AlertDialog::new(Some(&f.message), Some(&f.detail));
                dialog.add_response("ok", tr("_OK"));
                dialog.present(Some(&ctx.window));
            }
        }
        ctx.refresh();
    });
}

fn ask_create(ctx: &C) {
    let dialog = adw::AlertDialog::new(Some(tr("Create Snapshot")), Some(tr("Save the current state of the system. Your personal files in Home are not included.")));
    let entry = gtk::Entry::builder().placeholder_text(tr("Description, for example “Before installing drivers”")).activates_default(true).build();
    dialog.set_extra_child(Some(&entry));
    dialog.add_response("cancel", tr("_Cancel"));
    dialog.add_response("create", tr("_Create"));
    dialog.set_response_appearance("create", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("create"));
    dialog.set_close_response("cancel");
    let ctx = ctx.clone();
    glib::spawn_future_local(async move {
        let focus = entry.clone();
        glib::idle_add_local_once(move || {
            focus.grab_focus();
        });
        if dialog.clone().choose_future(Some(&ctx.window)).await == "create" {
            let text = entry.text().trim().to_string();
            let description = if text.is_empty() { tr("Manual snapshot").to_string() } else { text };
            run_request(&ctx, Request::SnapshotCreate { description }, false);
        }
    });
}

fn ask_delete(ctx: &C, s: &SnapshotInfo) {
    let dialog = adw::AlertDialog::new(
        Some(tr("Delete Snapshot?")),
        Some(&trf("“{description}” from {date} will be deleted permanently.", &[("description", &s.description), ("date", &format_date(&s.created))])),
    );
    dialog.add_response("cancel", tr("_Cancel"));
    dialog.add_response("delete", tr("_Delete"));
    dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
    dialog.set_close_response("cancel");
    let ctx = ctx.clone();
    let id = s.id.clone();
    glib::spawn_future_local(async move {
        if dialog.choose_future(Some(&ctx.window)).await == "delete" {
            run_request(&ctx, Request::SnapshotDelete { id }, false);
        }
    });
}

fn ask_restore(ctx: &C, s: &SnapshotInfo) {
    let dialog = adw::AlertDialog::new(
        Some(tr("Restore This Snapshot?")),
        Some(&trf(
            "The system will return to “{description}” from {date} after a restart. Your personal files in Home are not changed.\n\nThe current system is saved as a snapshot first, so this can be undone.",
            &[("description", &s.description), ("date", &format_date(&s.created))],
        )),
    );
    dialog.add_response("cancel", tr("_Cancel"));
    dialog.add_response("restore", tr("_Restore"));
    dialog.set_response_appearance("restore", adw::ResponseAppearance::Destructive);
    dialog.set_close_response("cancel");
    let ctx = ctx.clone();
    let id = s.id.clone();
    glib::spawn_future_local(async move {
        if dialog.choose_future(Some(&ctx.window)).await == "restore" {
            run_request(&ctx, Request::Rollback { id }, true);
        }
    });
}

fn ask_restart(ctx: &C, msg: &str) {
    let dialog = adw::AlertDialog::new(Some(tr("Restart to Finish Restoring")), Some(msg));
    dialog.add_response("later", tr("Restart _Later"));
    dialog.add_response("now", tr("Restart _Now"));
    dialog.set_response_appearance("now", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("now"));
    let ctx = ctx.clone();
    glib::spawn_future_local(async move {
        if dialog.choose_future(Some(&ctx.window)).await == "now" {
            spawn_detached(&["systemctl", "reboot"]);
        }
    });
}

pub fn run(app: &adw::Application) {
    load_css();
    let window = adw::ApplicationWindow::builder().application(app).title(tr("Snapshots")).default_width(720).default_height(680).build();
    let tv = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    let add = gtk::Button::builder().icon_name("list-add-symbolic").tooltip_text(tr("Create Snapshot")).build();
    let spinner = adw::Spinner::builder().visible(false).build();
    header.pack_start(&add);
    header.pack_end(&spinner);
    tv.add_top_bar(&header);

    let prefs = adw::PreferencesPage::new();
    let points = adw::PreferencesGroup::builder()
        .title(tr("Restore Points"))
        .description(tr("Snapshots save the state of the operating system. Restoring one brings the system back to that moment, which helps after a bad update or configuration change. Your personal files in Home are never touched."))
        .build();
    let list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).build();
    list.add_css_class("boxed-list");
    points.add(&list);
    prefs.add(&points);

    let auto = adw::PreferencesGroup::builder().title(tr("Automatic Snapshots")).build();
    let daily = adw::SwitchRow::builder().title(tr("Daily Snapshots")).subtitle(tr("Once a day while the computer is on")).build();
    let keep_daily = adw::SpinRow::with_range(1.0, 60.0, 1.0);
    keep_daily.set_title(tr("Daily Snapshots to Keep"));
    let pre_update = adw::SwitchRow::builder().title(tr("Before Software Updates")).subtitle(tr("Taken automatically when packages are installed, upgraded or removed")).build();
    let keep_pre_update = adw::SpinRow::with_range(1.0, 60.0, 1.0);
    keep_pre_update.set_title(tr("Update Snapshots to Keep"));
    for w in [daily.upcast_ref::<gtk::Widget>(), keep_daily.upcast_ref(), pre_update.upcast_ref(), keep_pre_update.upcast_ref()] {
        auto.add(w);
    }
    prefs.add(&auto);

    let storage = adw::PreferencesGroup::builder().title(tr("Storage")).build();
    let usage = adw::ActionRow::builder().title(tr("Disk Space")).build();
    usage.add_prefix(&icon("drive-harddisk-symbolic"));
    storage.add(&usage);
    let boot_hint = adw::ActionRow::builder()
        .title(tr("If MAN Does Not Start"))
        .subtitle(tr("Choose “Restore a MAN snapshot” in the boot menu to bring back a working system."))
        .build();
    boot_hint.add_prefix(&icon("dialog-information-symbolic"));
    storage.add(&boot_hint);
    prefs.add(&storage);

    let unavailable = adw::StatusPage::builder()
        .icon_name("com.man.Snapshots")
        .title(tr("Snapshots Are Not Available"))
        .description(tr("Snapshots work on MAN installed with the Btrfs layout created by the MAN installer. This system (for example a live session) does not use it."))
        .build();
    let stack = gtk::Stack::new();
    stack.add_named(&prefs, Some("list"));
    stack.add_named(&unavailable, Some("unavailable"));
    let banner = adw::Banner::builder()
        .title(tr("A restore is waiting: restart the computer to finish it"))
        .button_label(tr("Restart Now"))
        .build();
    banner.connect_button_clicked(|_| spawn_detached(&["systemctl", "reboot"]));
    tv.add_top_bar(&banner);
    let toasts = adw::ToastOverlay::new();
    toasts.set_child(Some(&stack));
    tv.set_content(Some(&toasts));
    window.set_content(Some(&tv));

    let ctx: C = Rc::new(Ctx {
        window: window.clone(),
        toasts,
        stack,
        list,
        spinner,
        add: add.clone(),
        banner,
        groups: vec![points.clone().upcast(), auto.clone().upcast()],
        usage,
        daily: daily.clone(),
        keep_daily: keep_daily.clone(),
        pre_update: pre_update.clone(),
        keep_pre_update: keep_pre_update.clone(),
        loading_config: Cell::new(false),
        config_timer: RefCell::new(None),
        busy: Cell::new(false),
    });
    {
        let ctx = ctx.clone();
        add.connect_clicked(move |_| ask_create(&ctx));
    }
    for sw in [&daily, &pre_update] {
        let ctx = ctx.clone();
        sw.connect_active_notify(move |_| ctx.schedule_config_save());
    }
    for spin in [&keep_daily, &keep_pre_update] {
        let ctx = ctx.clone();
        spin.connect_value_notify(move |_| ctx.schedule_config_save());
    }
    {
        let ctx = ctx.clone();
        window.connect_close_request(move |_| {
            if ctx.busy.get() {
                ctx.toast(tr("Please wait until the current operation finishes."));
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
    }
    ctx.refresh();
    window.present();
}

pub fn main() -> glib::ExitCode {
    crate::i18n::init();
    let app = adw::Application::builder().application_id("com.man.Snapshots").build();
    app.connect_activate(|app| {
        if let Some(w) = app.active_window() {
            w.present();
        } else {
            run(app);
        }
    });
    app.run()
}
