//! Marquee on MAN: starting the app asks whether to open the media centre or to
//! share media with this network, and the libraries are managed here. The files
//! are served by `marquee-share` (a user service, so sharing goes on with this
//! window closed), which watches the libraries file this window writes.
use crate::i18n::{tr, trf};
use adw::prelude::*;
use gtk::{gio, glib};
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;

const APP_ID: &str = "com.man.Marquee";
const MEDIA_CENTRE: &str = "/usr/libexec/marquee/marquee";
const SERVICE: &str = "marquee-share.service";
const ICON: &str = "marquee";

const VIDEO: &[&str] = &["mp4", "m4v", "mov", "mkv", "avi", "webm", "mpg", "mpeg", "wmv", "ts", "flv", "ogv"];
const AUDIO: &[&str] = &["mp3", "m4a", "m4b", "aac", "flac", "ogg", "oga", "opus", "wav", "aif", "aiff"];
const PHOTO: &[&str] = &["jpg", "jpeg", "png", "gif", "webp"];
/// The kinds `marquee-share` knows, in the order of the Type row.
const KINDS: [&str; 3] = ["movies", "music", "photos"];

// ──────────────────────────── Libraries ─────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Library {
    id: String,
    name: String,
    kind: String,
    paths: Vec<PathBuf>,
}

fn libraries_file() -> PathBuf {
    glib::user_config_dir().join("marquee-share").join("libraries.json")
}

fn parse_libraries(text: &str) -> Vec<Library> {
    serde_json::from_str(text).unwrap_or_default()
}

fn load_libraries() -> Vec<Library> {
    std::fs::read_to_string(libraries_file()).map(|s| parse_libraries(&s)).unwrap_or_default()
}

/// Written to a new file first: the service never reads half a list.
fn save_libraries(libraries: &[Library]) -> Result<(), String> {
    let path = libraries_file();
    let dir = path.parent().ok_or("no folder")?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(libraries).map_err(|e| e.to_string())?;
    let new = dir.join("libraries.json.new");
    std::fs::write(&new, text).map_err(|e| e.to_string())?;
    std::fs::rename(&new, &path).map_err(|e| e.to_string())
}

fn next_id(libraries: &[Library]) -> String {
    libraries.iter().filter_map(|l| l.id.parse::<u32>().ok()).max().map_or(0, |m| m + 1).to_string()
}

fn extensions(kind: &str) -> &'static [&'static str] {
    match kind {
        "music" => AUDIO,
        "photos" => PHOTO,
        _ => VIDEO,
    }
}

fn kind_label(kind: &str) -> &'static str {
    match kind {
        "music" => tr("Music"),
        "photos" => tr("Photos"),
        _ => tr("Movies"),
    }
}

fn kind_icon(kind: &str) -> &'static str {
    match kind {
        "music" => "folder-music-symbolic",
        "photos" => "folder-pictures-symbolic",
        _ => "folder-videos-symbolic",
    }
}

/// `/home/ana/Videos` → `~/Videos`, in the lists only.
fn short_path(path: &Path) -> String {
    match path.strip_prefix(glib::home_dir()) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".into(),
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}

fn file_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.display().to_string())
}

// ───────────────────────────── Sharing ──────────────────────────────

fn systemctl(args: &[&str]) -> bool {
    Command::new("systemctl").arg("--user").args(args).stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success())
}

fn is_sharing() -> bool {
    systemctl(&["is-active", "--quiet", SERVICE])
}

/// On also means at every login, until it is turned off here.
fn set_sharing(on: bool) -> bool {
    if on {
        systemctl(&["enable", "--now", SERVICE])
    } else {
        systemctl(&["disable", "--now", SERVICE])
    }
}

/// This computer's address on the network. No packet is sent: connecting a UDP
/// socket only makes the system choose the route.
fn local_address() -> Option<String> {
    let socket = std::net::UdpSocket::bind(("0.0.0.0", 0)).ok()?;
    socket.connect(("192.0.2.1", 9)).ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_loopback() && !ip.is_unspecified()).then(|| ip.to_string())
}

// ───────────────────────────── The window ───────────────────────────

struct Manager {
    window: adw::ApplicationWindow,
    toasts: adw::ToastOverlay,
    stack: gtk::Stack,
    switch: adw::SwitchRow,
    group: adw::PreferencesGroup,
    rows: RefCell<Vec<adw::ActionRow>>,
    libraries: RefCell<Vec<Library>>,
    /// Set while the code moves the switch, so that only a person's click acts.
    updating: Cell<bool>,
}

impl Manager {
    fn toast(&self, text: &str) {
        self.toasts.add_toast(adw::Toast::new(text));
    }

    fn sync_switch(&self) {
        let on = is_sharing();
        self.updating.set(true);
        self.switch.set_active(on);
        self.updating.set(false);
        let name = glib::host_name();
        self.switch.set_subtitle(&match (on, local_address()) {
            (true, Some(address)) => trf("Sharing as “{name}” at {address}", &[("name", &name), ("address", &address)]),
            (true, None) => trf("Sharing as “{name}”. This computer is not connected to a network yet.", &[("name", &name)]),
            (false, _) => tr("Other computers cannot see your libraries.").to_string(),
        });
    }

    fn start_sharing(self: &Rc<Self>) -> bool {
        let ok = set_sharing(true);
        // A failed start shows only a moment later.
        let this = self.clone();
        glib::timeout_add_seconds_local_once(1, move || {
            if !is_sharing() && this.switch.is_active() {
                this.toast(tr("Sharing could not start."));
            }
            this.sync_switch();
        });
        self.sync_switch();
        ok
    }

    fn store(&self) -> bool {
        match save_libraries(&self.libraries.borrow()) {
            Ok(()) => true,
            Err(e) => {
                eprintln!("marquee: saving the libraries: {e}");
                self.toast(tr("The libraries could not be saved."));
                false
            }
        }
    }

    fn refresh(self: &Rc<Self>) {
        for row in self.rows.borrow_mut().drain(..) {
            self.group.remove(&row);
        }
        let libraries = self.libraries.borrow().clone();
        self.stack.set_visible_child_name(if libraries.is_empty() { "empty" } else { "list" });
        for library in libraries {
            let row = adw::ActionRow::builder().title(glib::markup_escape_text(&library.name).as_str()).activatable(true).build();
            let places: Vec<String> = library.paths.iter().map(|p| short_path(p)).collect();
            row.set_subtitle(&glib::markup_escape_text(&format!("{} · {}", kind_label(&library.kind), places.join(", "))));
            row.set_subtitle_lines(1);
            row.add_prefix(&gtk::Image::from_icon_name(kind_icon(&library.kind)));
            let delete = gtk::Button::builder().icon_name("user-trash-symbolic").valign(gtk::Align::Center).tooltip_text(tr("Stop Sharing")).build();
            delete.add_css_class("flat");
            row.add_suffix(&delete);
            row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
            {
                let (this, library) = (self.clone(), library.clone());
                row.connect_activated(move |_| this.edit(Some(library.clone())));
            }
            {
                let this = self.clone();
                delete.connect_clicked(move |_| this.confirm_delete(&library));
            }
            self.group.add(&row);
            self.rows.borrow_mut().push(row);
        }
        self.sync_switch();
    }

    fn confirm_delete(self: &Rc<Self>, library: &Library) {
        let dialog = adw::AlertDialog::new(
            Some(&trf("Stop Sharing “{name}”?", &[("name", &library.name)])),
            Some(tr("The files stay on this computer. Other computers just cannot see this library any more.")),
        );
        dialog.add_response("cancel", tr("_Cancel"));
        dialog.add_response("stop", tr("_Stop Sharing"));
        dialog.set_response_appearance("stop", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        let (this, id) = (self.clone(), library.id.clone());
        dialog.connect_response(None, move |_, response| {
            if response != "stop" {
                return;
            }
            this.libraries.borrow_mut().retain(|l| l.id != id);
            if this.store() {
                // Nothing left to share: nothing announces this computer either.
                if this.libraries.borrow().is_empty() {
                    set_sharing(false);
                }
            }
            this.refresh();
        });
        dialog.present(Some(&self.window));
    }

    fn save_edit(self: &Rc<Self>, old: Option<&Library>, name: String, kind: &str, paths: Vec<PathBuf>) {
        let mut libraries = self.libraries.borrow_mut();
        let id = old.map_or_else(|| next_id(&libraries), |l| l.id.clone());
        let library = Library { id, name: name.clone(), kind: kind.to_string(), paths };
        match old.and_then(|o| libraries.iter().position(|l| l.id == o.id)) {
            Some(i) => libraries[i] = library,
            None => libraries.push(library),
        }
        drop(libraries);
        if self.store() {
            if old.is_none() {
                // Creating a shared library means sharing it.
                if self.start_sharing() {
                    self.toast(&trf("“{name}” is shared on this network.", &[("name", &name)]));
                }
            }
        }
        self.refresh();
    }

    /// The Library page, for a new library or an existing one.
    fn edit(self: &Rc<Self>, old: Option<Library>) {
        let dialog = adw::Dialog::builder().content_width(480).content_height(520).title(if old.is_some() { tr("Edit Library") } else { tr("New Library") }).build();
        let page = adw::PreferencesPage::new();

        let about = adw::PreferencesGroup::new();
        let name = adw::EntryRow::builder().title(tr("Name")).build();
        let kinds = gtk::StringList::new(&[tr("Movies"), tr("Music"), tr("Photos")]);
        let kind = adw::ComboRow::builder().title(tr("Type")).model(&kinds).build();
        about.add(&name);
        about.add(&kind);
        page.add(&about);

        let places = adw::PreferencesGroup::builder().title(tr("Folders and Files")).build();
        let buttons = gtk::Box::builder().spacing(6).build();
        let add_folder = gtk::Button::builder().child(&adw::ButtonContent::builder().icon_name("folder-new-symbolic").label(tr("Add Folder…")).build()).build();
        let add_files = gtk::Button::builder().child(&adw::ButtonContent::builder().icon_name("document-new-symbolic").label(tr("Add Files…")).build()).build();
        add_folder.add_css_class("flat");
        add_files.add_css_class("flat");
        buttons.append(&add_folder);
        buttons.append(&add_files);
        places.set_header_suffix(Some(&buttons));
        page.add(&places);

        let cancel = gtk::Button::with_mnemonic(tr("_Cancel"));
        let save = gtk::Button::with_mnemonic(if old.is_some() { tr("_Save") } else { tr("C_reate") });
        save.add_css_class("suggested-action");
        let header = adw::HeaderBar::builder().show_start_title_buttons(false).show_end_title_buttons(false).build();
        header.pack_start(&cancel);
        header.pack_end(&save);
        let view = adw::ToolbarView::new();
        view.add_top_bar(&header);
        view.set_content(Some(&page));
        dialog.set_child(Some(&view));

        struct Edit {
            places: adw::PreferencesGroup,
            rows: RefCell<Vec<adw::ActionRow>>,
            paths: RefCell<Vec<PathBuf>>,
            name: adw::EntryRow,
            save: gtk::Button,
        }
        fn check(e: &Edit) {
            e.save.set_sensitive(!e.name.text().trim().is_empty() && !e.paths.borrow().is_empty());
        }
        fn show_places(e: &Rc<Edit>) {
            for row in e.rows.borrow_mut().drain(..) {
                e.places.remove(&row);
            }
            let paths = e.paths.borrow().clone();
            if paths.is_empty() {
                let row = adw::ActionRow::builder().title(tr("Nothing added yet")).build();
                row.add_css_class("dim-label");
                e.places.add(&row);
                e.rows.borrow_mut().push(row);
            }
            for path in paths {
                let row = adw::ActionRow::builder().title(glib::markup_escape_text(&file_name(&path)).as_str()).subtitle(glib::markup_escape_text(&short_path(&path)).as_str()).build();
                row.add_prefix(&gtk::Image::from_icon_name(if path.is_dir() { "folder-symbolic" } else { "text-x-generic-symbolic" }));
                let remove = gtk::Button::builder().icon_name("list-remove-symbolic").valign(gtk::Align::Center).tooltip_text(tr("Remove")).build();
                remove.add_css_class("flat");
                let e2 = e.clone();
                remove.connect_clicked(move |_| {
                    e2.paths.borrow_mut().retain(|p| *p != path);
                    show_places(&e2);
                });
                row.add_suffix(&remove);
                e.places.add(&row);
                e.rows.borrow_mut().push(row);
            }
            check(e);
        }
        fn add_paths(e: &Rc<Edit>, picked: Vec<PathBuf>) {
            {
                let mut paths = e.paths.borrow_mut();
                for path in &picked {
                    if !paths.contains(path) {
                        paths.push(path.clone());
                    }
                }
            }
            if e.name.text().trim().is_empty() {
                if let Some(first) = picked.first().filter(|p| p.is_dir()) {
                    e.name.set_text(&file_name(first));
                }
            }
            show_places(e);
        }
        fn from_list(files: &gio::ListModel) -> Vec<PathBuf> {
            (0..files.n_items()).filter_map(|i| files.item(i)?.downcast::<gio::File>().ok()?.path()).collect()
        }

        let edit = Rc::new(Edit { places, rows: RefCell::default(), paths: RefCell::new(old.as_ref().map(|l| l.paths.clone()).unwrap_or_default()), name: name.clone(), save: save.clone() });
        if let Some(library) = &old {
            name.set_text(&library.name);
            kind.set_selected(KINDS.iter().position(|k| *k == library.kind).unwrap_or(0) as u32);
        }
        show_places(&edit);
        {
            let e = edit.clone();
            name.connect_changed(move |_| check(&e));
        }
        {
            let dialog = dialog.clone();
            cancel.connect_clicked(move |_| {
                dialog.close();
            });
        }
        {
            let (e, parent) = (edit.clone(), self.window.clone());
            add_folder.connect_clicked(move |_| {
                let e = e.clone();
                gtk::FileDialog::builder().title(tr("Add Folder…")).modal(true).build().select_multiple_folders(Some(&parent), gio::Cancellable::NONE, move |result| {
                    if let Ok(files) = result {
                        add_paths(&e, from_list(&files));
                    }
                });
            });
        }
        {
            let (e, parent, kind) = (edit.clone(), self.window.clone(), kind.clone());
            add_files.connect_clicked(move |_| {
                let chosen = KINDS[(kind.selected() as usize).min(2)];
                let filter = gtk::FileFilter::new();
                filter.set_name(Some(kind_label(chosen)));
                for extension in extensions(chosen) {
                    filter.add_suffix(extension);
                }
                let filters = gio::ListStore::new::<gtk::FileFilter>();
                filters.append(&filter);
                let e = e.clone();
                gtk::FileDialog::builder().title(tr("Add Files…")).modal(true).filters(&filters).default_filter(&filter).build().open_multiple(Some(&parent), gio::Cancellable::NONE, move |result| {
                    if let Ok(files) = result {
                        add_paths(&e, from_list(&files));
                    }
                });
            });
        }
        {
            let (this, dialog2, e, kind) = (self.clone(), dialog.clone(), edit.clone(), kind.clone());
            save.connect_clicked(move |_| {
                let chosen = KINDS[(kind.selected() as usize).min(2)];
                this.save_edit(old.as_ref(), e.name.text().trim().to_string(), chosen, e.paths.borrow().clone());
                dialog2.close();
            });
        }
        dialog.present(Some(&self.window));
        name.grab_focus();
    }
}

fn open_manager(app: &adw::Application) {
    let window = adw::ApplicationWindow::builder().application(app).title(tr("Shared Libraries")).icon_name(ICON).default_width(640).default_height(620).width_request(360).height_request(360).build();

    let empty_button = gtk::Button::builder().label(tr("Create Library")).halign(gtk::Align::Center).build();
    empty_button.add_css_class("pill");
    empty_button.add_css_class("suggested-action");
    let empty = adw::StatusPage::builder()
        .icon_name("network-server-symbolic")
        .title(tr("Share Your Media"))
        .description(tr("Choose folders or files, and other computers on this network can play them in Marquee."))
        .child(&empty_button)
        .build();

    let page = adw::PreferencesPage::new();
    let sharing = adw::PreferencesGroup::builder().description(tr("Anyone on this network can play what you share here, without a password, so share only on networks you trust. On the other computer, open Marquee and go to Network.")).build();
    let switch = adw::SwitchRow::builder().title(tr("Share on This Network")).build();
    sharing.add(&switch);
    page.add(&sharing);
    let group = adw::PreferencesGroup::builder().title(tr("Libraries")).build();
    page.add(&group);

    let stack = gtk::Stack::new();
    stack.add_named(&empty, Some("empty"));
    stack.add_named(&page, Some("list"));

    let add = gtk::Button::builder().icon_name("list-add-symbolic").tooltip_text(tr("New Library")).build();
    let header = adw::HeaderBar::new();
    header.pack_start(&add);
    let toasts = adw::ToastOverlay::new();
    toasts.set_child(Some(&stack));
    let view = adw::ToolbarView::new();
    view.add_top_bar(&header);
    view.set_content(Some(&toasts));
    window.set_content(Some(&view));

    let manager = Rc::new(Manager { window: window.clone(), toasts, stack, switch: switch.clone(), group, rows: RefCell::default(), libraries: RefCell::new(load_libraries()), updating: Cell::new(false) });
    {
        let m = manager.clone();
        add.connect_clicked(move |_| m.edit(None));
    }
    {
        let m = manager.clone();
        empty_button.connect_clicked(move |_| m.edit(None));
    }
    {
        let m = manager.clone();
        switch.connect_active_notify(move |s| {
            if m.updating.get() {
                return;
            }
            if s.is_active() {
                m.start_sharing();
            } else {
                set_sharing(false);
                m.sync_switch();
            }
        });
    }
    manager.refresh();
    window.present();
}

// ───────────────────────── The first question ───────────────────────

fn open_media_centre() -> bool {
    match Command::new(MEDIA_CENTRE).stdin(Stdio::null()).spawn() {
        Ok(_) => true,
        Err(e) => {
            eprintln!("marquee: {MEDIA_CENTRE}: {e}");
            false
        }
    }
}

fn ask(app: &adw::Application) {
    let window: gtk::Window = adw::Window::builder().application(app).resizable(false).icon_name(ICON).title("Marquee").build().upcast();
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
    let share_label = if load_libraries().is_empty() { tr("Create _Shared Library") } else { tr("Shared _Libraries") };
    let app2 = app.clone();
    let buttons: Vec<crate::winprogram::Btn> = vec![
        (tr("_Open Marquee"), "", Box::new(|w, _| {
            open_media_centre();
            w.close();
        })),
        (share_label, "", Box::new(move |w, _| {
            open_manager(&app2);
            w.close();
        })),
    ];
    crate::winprogram::show(&window, ICON, tr("Welcome to Marquee"), tr("Would you like to open Marquee, or share your movies, music and photos with other computers on this network?"), buttons, false);
    window.present();
}

pub fn main() -> glib::ExitCode {
    crate::i18n::init();
    let arg = std::env::args().nth(1);
    if arg.as_deref() == Some("--open") {
        return if open_media_centre() { glib::ExitCode::SUCCESS } else { glib::ExitCode::FAILURE };
    }
    let share = arg.as_deref() == Some("--share");
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_startup(|_| crate::winprogram::load_css());
    app.connect_activate(move |app| {
        if let Some(window) = app.active_window() {
            window.present();
        } else if share {
            open_manager(app);
        } else {
            ask(app);
        }
    });
    app.run_with_args::<&str>(&[])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib(id: &str) -> Library {
        Library { id: id.into(), name: format!("L{id}"), kind: "movies".into(), paths: vec![PathBuf::from("/m")] }
    }

    #[test]
    fn ids_never_repeat_a_living_one() {
        assert_eq!(next_id(&[]), "0");
        assert_eq!(next_id(&[lib("0"), lib("4")]), "5");
        assert_eq!(next_id(&[lib("x")]), "0");
    }

    #[test]
    fn reads_what_the_service_reads() {
        let text = serde_json::to_string(&[lib("3")]).unwrap();
        assert!(text.contains("\"paths\":[\"/m\"]"));
        assert_eq!(parse_libraries(&text), vec![lib("3")]);
        assert!(parse_libraries("not json").is_empty());
    }

    #[test]
    fn kinds_have_their_files() {
        assert!(extensions("movies").contains(&"mkv"));
        assert!(extensions("music").contains(&"flac"));
        assert!(extensions("photos").contains(&"jpg"));
    }
}
