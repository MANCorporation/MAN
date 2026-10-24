//! Small building blocks shared by the installer and the snapshot manager.
use crate::i18n::{tr};
use adw::prelude::*;
use gtk::glib;
use std::rc::Rc;

const CSS: &str = r#"
.hero-icon { -gtk-icon-size: 128px; }
.step-title { font-weight: 800; font-size: 1.6em; }
.install-progress trough, .install-progress progress { min-height: 10px; border-radius: 99px; }
.log-view { font-family: monospace; font-size: 0.85em; padding: 12px; border-radius: 12px; }
.choice-hint { padding: 0 12px; }
.summary-warning { padding: 12px 16px; border-radius: 12px; }
.done-page image.icon { color: var(--success-color); }
"#;

pub fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(CSS);
    if let Some(display) = gtk::gdk::Display::default() {
        #[allow(deprecated)]
        gtk::style_context_add_provider_for_display(&display, &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }
}

pub fn pill(label: &str, class: Option<&str>) -> gtk::Button {
    let b = gtk::Button::builder().label(label).use_underline(true).halign(gtk::Align::Center).build();
    b.add_css_class("pill");
    if let Some(c) = class {
        b.add_css_class(c);
    }
    b
}

/// A navigation page: header bar on top, scrollable clamped content and an
/// optional bottom bar with the primary action.
pub fn page(title: &str, tag: &str, content: &impl IsA<gtk::Widget>, bottom: Option<&gtk::Widget>) -> adw::NavigationPage {
    let tv = adw::ToolbarView::new();
    tv.add_top_bar(&adw::HeaderBar::new());
    tv.set_content(Some(content));
    if let Some(b) = bottom {
        let bar = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_top(12)
            .margin_bottom(18)
            .margin_start(12)
            .margin_end(12)
            .spacing(8)
            .build();
        bar.append(b);
        tv.add_bottom_bar(&bar);
    }
    adw::NavigationPage::builder().title(title).tag(tag).child(&tv).build()
}

pub fn label(text: &str, classes: &[&str]) -> gtk::Label {
    let l = gtk::Label::builder().label(text).wrap(true).justify(gtk::Justification::Center).build();
    for c in classes {
        l.add_css_class(c);
    }
    l
}

pub fn icon(name: &str) -> gtk::Image {
    gtk::Image::from_icon_name(name)
}

/// Searchable single-choice dialog for long lists (time zones, layouts).
pub fn chooser(
    parent: &impl IsA<gtk::Widget>,
    title: &str,
    items: Rc<Vec<(String, String, String)>>,
    current: &str,
    on_select: impl Fn(&str) + 'static,
) {
    let dialog = adw::Dialog::builder().title(title).content_width(460).content_height(560).build();
    let tv = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    tv.add_top_bar(&header);
    let search = gtk::SearchEntry::builder().placeholder_text(tr("Search")).hexpand(true).build();
    let search_bar = gtk::Box::builder().margin_start(12).margin_end(12).margin_bottom(6).build();
    search_bar.append(&search);
    tv.add_top_bar(&search_bar);

    let list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).valign(gtk::Align::Start).build();
    list.add_css_class("boxed-list");
    let mut current_row = None;
    for (id, title, subtitle) in items.iter() {
        let row = adw::ActionRow::builder().title(glib::markup_escape_text(title)).activatable(true).build();
        if !subtitle.is_empty() {
            row.set_subtitle(&glib::markup_escape_text(subtitle));
        }
        row.set_widget_name(id);
        if id == current {
            row.add_suffix(&icon("object-select-symbolic"));
            current_row = Some(row.clone());
        }
        list.append(&row);
    }
    let query = Rc::new(std::cell::RefCell::new(String::new()));
    {
        let query = query.clone();
        let items = items.clone();
        list.set_filter_func(move |row| {
            let q = query.borrow();
            if q.is_empty() {
                return true;
            }
            let idx = row.index() as usize;
            let (id, t, s) = &items[idx];
            [id, t, s].iter().any(|x| x.to_lowercase().contains(q.as_str()))
        });
    }
    {
        let list = list.clone();
        search.connect_search_changed(move |s| {
            *query.borrow_mut() = s.text().to_lowercase();
            list.invalidate_filter();
        });
    }
    {
        let dialog = dialog.clone();
        list.connect_row_activated(move |_, row| {
            on_select(&row.widget_name());
            dialog.close();
        });
    }
    let clamp = adw::Clamp::builder().maximum_size(560).child(&list).margin_start(12).margin_end(12).margin_top(6).margin_bottom(12).build();
    let scroll = gtk::ScrolledWindow::builder().hscrollbar_policy(gtk::PolicyType::Never).vexpand(true).child(&clamp).build();
    tv.set_content(Some(&scroll));
    dialog.set_child(Some(&tv));
    dialog.present(Some(parent));
    search.grab_focus();
    if let Some(row) = current_row {
        // Scroll the current choice into view, then give typing back to search.
        glib::idle_add_local_once(move || {
            row.grab_focus();
            search.grab_focus();
        });
    }
}

/// Dialog showing a long technical text, with a copy button.
pub fn details_dialog(parent: &impl IsA<gtk::Widget>, title: &str, text: &str) {
    let dialog = adw::Dialog::builder().title(title).content_width(640).content_height(480).build();
    let tv = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    let copy = gtk::Button::from_icon_name("edit-copy-symbolic");
    copy.set_tooltip_text(Some(tr("Copy to Clipboard")));
    header.pack_start(&copy);
    tv.add_top_bar(&header);
    let view = gtk::TextView::builder().editable(false).wrap_mode(gtk::WrapMode::WordChar).build();
    view.add_css_class("log-view");
    view.buffer().set_text(text);
    let text = text.to_string();
    copy.connect_clicked(move |b| b.clipboard().set_text(&text));
    let scroll = gtk::ScrolledWindow::builder().vexpand(true).child(&view).margin_start(12).margin_end(12).margin_bottom(12).build();
    tv.set_content(Some(&scroll));
    dialog.set_child(Some(&tv));
    dialog.present(Some(parent));
}

/// (id, title, subtitle) for every time zone in tzdata.
pub fn timezones() -> Vec<(String, String, String)> {
    let mut v: Vec<(String, String, String)> = std::fs::read_to_string("/usr/share/zoneinfo/zone1970.tab")
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.split('\t').nth(2))
        .map(|tz| {
            let (region, city) = tz.rsplit_once('/').unwrap_or(("", tz));
            (tz.to_string(), city.replace('_', " "), region.replace('_', " ").replace('/', " · "))
        })
        .collect();
    v.push(("Etc/UTC".into(), "UTC".into(), tr("Coordinated Universal Time").into()));
    v.sort_by(|a, b| a.1.cmp(&b.1));
    v.dedup_by(|a, b| a.0 == b.0);
    v
}

/// (id, title, subtitle) for keyboard layouts and variants from XKB.
/// Variant ids use GNOME's `layout+variant` form.
pub fn keyboards() -> Vec<(String, String, String)> {
    let text = std::fs::read_to_string("/usr/share/X11/xkb/rules/evdev.lst").unwrap_or_default();
    let mut section = "";
    let mut v = vec![];
    for line in text.lines() {
        if let Some(s) = line.strip_prefix("! ") {
            section = s.trim();
            continue;
        }
        let line = line.trim();
        let Some((id, desc)) = line.split_once(char::is_whitespace) else { continue };
        let desc = desc.trim();
        match section {
            "layout" => v.push((id.to_string(), desc.to_string(), String::new())),
            "variant" => {
                if let Some((layout, name)) = desc.split_once(": ") {
                    v.push((format!("{layout}+{id}"), name.to_string(), String::new()));
                }
            }
            _ => {}
        }
    }
    if v.is_empty() {
        v.push(("us".into(), tr("English (US)").into(), String::new()));
    }
    v.sort_by(|a, b| a.1.cmp(&b.1));
    v
}

pub fn title_for(items: &[(String, String, String)], id: &str) -> String {
    items.iter().find(|i| i.0 == id).map(|i| i.1.clone()).unwrap_or_else(|| id.to_string())
}

pub fn spawn_detached(argv: &[&str]) {
    let os: Vec<&std::ffi::OsStr> = argv.iter().map(std::ffi::OsStr::new).collect();
    let _ = gtk::gio::Subprocess::newv(&os, gtk::gio::SubprocessFlags::NONE);
}

/// Suggest a username from a full name: first word, lowercase ASCII.
pub fn derive_username(full: &str) -> String {
    let first = full.split_whitespace().next().unwrap_or("").to_lowercase();
    let mut u: String = first
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' | 'å' => 'a',
            'č' | 'ç' => 'c',
            'ď' => 'd',
            'é' | 'è' | 'ě' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ň' | 'ñ' => 'n',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ř' => 'r',
            'š' | 'ß' => 's',
            'ť' => 't',
            'ú' | 'ů' | 'ù' | 'ü' | 'û' => 'u',
            'ý' | 'ÿ' => 'y',
            'ž' => 'z',
            c => c,
        })
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        .collect();
    if u.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        u.insert(0, 'u');
    }
    u.truncate(32);
    u
}

pub fn set_error(w: &impl IsA<gtk::Widget>, bad: bool) {
    if bad {
        w.add_css_class("error");
    } else {
        w.remove_css_class("error");
    }
}

#[cfg(test)]
mod tests {
    use super::derive_username;

    #[test]
    fn usernames_from_names() {
        assert_eq!(derive_username("Aleš Trnka"), "ales");
        assert_eq!(derive_username("  Žofie  "), "zofie");
        assert_eq!(derive_username("3D Artist"), "u3d");
        assert_eq!(derive_username(""), "");
    }
}

