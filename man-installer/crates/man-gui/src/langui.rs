//! Language and country choosers shared by MAN Setup and the installer, and
//! the drifting "Welcome" in every language behind the globe.
use crate::i18n::{tr};
use adw::prelude::*;
use gtk::glib;
use man_common::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const CSS: &str = r#"
.welcome-word { font-weight: 700; color: alpha(currentColor, 0.16); }
.welcome-size-0 { font-size: 2.1em; }
.welcome-size-1 { font-size: 1.35em; }
.welcome-size-2 { font-size: 1.75em; }
.welcome-size-3 { font-size: 1.15em; }
.globe-hero { -gtk-icon-size: 128px; }
.welcome-title { font-weight: 800; font-size: 2.2em; }
.flag { font-size: 1.6em; }
"#;

pub fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(CSS);
    if let Some(display) = gtk::gdk::Display::default() {
        #[allow(deprecated)]
        gtk::style_context_add_provider_for_display(&display, &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }
}

/// Rows of "Welcome" in every language, drifting left and right.
pub fn welcome_marquee(height: i32) -> gtk::Widget {
    const ROWS: usize = 4;
    let fixed = gtk::Fixed::builder().hexpand(true).height_request(height).can_target(false).build();
    fixed.set_overflow(gtk::Overflow::Hidden);
    let words: Vec<&str> = {
        let mut seen = vec![];
        for l in LANGUAGES {
            if !seen.contains(&l.welcome) {
                seen.push(l.welcome);
            }
        }
        seen
    };
    let mut rows = vec![];
    for r in 0..ROWS {
        let row = gtk::Box::builder().spacing(42).build();
        // Each row starts at a different word; the list is repeated so the
        // row can wrap around without a gap.
        for _ in 0..2 {
            for i in 0..words.len() {
                let w = words[(i + r * 11) % words.len()];
                let l = gtk::Label::new(Some(w));
                l.add_css_class("welcome-word");
                l.add_css_class(&format!("welcome-size-{r}"));
                row.append(&l);
            }
        }
        let y = (height as f64 / ROWS as f64) * r as f64 + 4.0;
        fixed.put(&row, 0.0, y);
        rows.push((row, y));
    }
    let widths: Rc<RefCell<Vec<f64>>> = Rc::new(RefCell::new(vec![]));
    let start = Rc::new(Cell::new(0i64));
    fixed.add_tick_callback(move |fixed, clock| {
        let now = clock.frame_time();
        if start.get() == 0 {
            start.set(now);
        }
        let t = (now - start.get()) as f64 / 1_000_000.0;
        let mut widths = widths.borrow_mut();
        if widths.is_empty() {
            *widths = rows.iter().map(|(row, _)| row.measure(gtk::Orientation::Horizontal, -1).1 as f64 / 2.0).collect();
        }
        for (i, (row, y)) in rows.iter().enumerate() {
            let half = widths[i].max(1.0);
            let speed = [22.0, 34.0, 16.0, 28.0][i % 4];
            let offset = (t * speed + i as f64 * 137.0) % half;
            // Alternate directions.
            let x = if i % 2 == 0 { -offset } else { offset - half };
            fixed.move_(row, x, *y);
        }
        glib::ControlFlow::Continue
    });
    // The rows are much wider than the window: show a window onto them that
    // does not ask for their width.
    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::External)
        .vscrollbar_policy(gtk::PolicyType::Never)
        .height_request(height)
        .hexpand(true)
        .can_target(false)
        .child(&fixed)
        .build()
        .upcast()
}

/// Big globe with the chosen language's "Welcome" over the drifting words.
pub struct Hero {
    pub widget: gtk::Overlay,
    pub title: gtk::Label,
}

pub fn hero(welcome: &str) -> Hero {
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&welcome_marquee(230)));
    let center = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(6).halign(gtk::Align::Center).valign(gtk::Align::Center).build();
    let globe = gtk::Image::from_icon_name("com.man.Globe");
    globe.add_css_class("globe-hero");
    let title = gtk::Label::new(Some(welcome));
    title.add_css_class("welcome-title");
    center.append(&globe);
    center.append(&title);
    overlay.add_overlay(&center);
    Hero { widget: overlay, title }
}

/// Fade the hero title to another word.
pub fn set_hero_title(label: &gtk::Label, text: &str) {
    if label.label() == text {
        return;
    }
    let out = adw::TimedAnimation::new(label, 1.0, 0.0, 140, adw::PropertyAnimationTarget::new(label, "opacity"));
    let label2 = label.clone();
    let text = text.to_string();
    out.connect_done(move |_| {
        label2.set_label(&text);
        adw::TimedAnimation::new(&label2, 0.0, 1.0, 220, adw::PropertyAnimationTarget::new(&label2, "opacity")).play();
    });
    out.play();
}

fn search_entry(placeholder: &str) -> gtk::SearchEntry {
    gtk::SearchEntry::builder().placeholder_text(placeholder).hexpand(true).build()
}

fn boxed_list() -> gtk::ListBox {
    let l = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).valign(gtk::Align::Start).build();
    l.add_css_class("boxed-list");
    l
}

fn fold(s: &str) -> String {
    s.to_lowercase()
}

/// A searchable single-choice list. `items`: (id, title, subtitle, prefix,
/// extra search text).
/// The chooser widgets; `selected` is the row chosen when it was built.
pub struct Chooser {
    pub search: gtk::SearchEntry,
    pub list: gtk::ListBox,
    pub selected: Option<adw::ActionRow>,
}

fn choice_list(items: Vec<(String, String, String, Option<String>, String)>, selected: &str, placeholder: &str, on_select: Rc<dyn Fn(&str)>) -> Chooser {
    let search = search_entry(placeholder);
    let list = boxed_list();
    let checks: Rc<RefCell<Vec<(String, gtk::Image)>>> = Rc::new(RefCell::new(vec![]));
    let mut selected_row = None;
    let haystacks: Rc<Vec<String>> = Rc::new(items.iter().map(|(id, t, s, _, extra)| fold(&format!("{id} {t} {s} {extra}"))).collect());
    for (id, title, subtitle, prefix, _) in &items {
        let row = adw::ActionRow::builder().title(glib::markup_escape_text(title)).activatable(true).build();
        if !subtitle.is_empty() && subtitle != title {
            row.set_subtitle(&glib::markup_escape_text(subtitle));
        }
        if let Some(p) = prefix {
            let l = gtk::Label::new(Some(p));
            l.add_css_class("flag");
            row.add_prefix(&l);
        }
        let check = gtk::Image::from_icon_name("object-select-symbolic");
        check.add_css_class("accent");
        check.set_visible(id == selected);
        if id == selected {
            selected_row = Some(row.clone());
        }
        row.add_suffix(&check);
        checks.borrow_mut().push((id.clone(), check));
        let id2 = id.clone();
        let checks2 = checks.clone();
        let on_select = on_select.clone();
        row.connect_activated(move |_| {
            for (cid, img) in checks2.borrow().iter() {
                img.set_visible(*cid == id2);
            }
            on_select(&id2);
        });
        list.append(&row);
    }
    {
        let haystacks = haystacks.clone();
        let search2 = search.clone();
        list.set_filter_func(move |row| {
            let q = fold(search2.text().as_str());
            q.is_empty() || haystacks.get(row.index() as usize).is_some_and(|h| h.contains(q.trim()))
        });
    }
    {
        let list = list.clone();
        search.connect_search_changed(move |_| list.invalidate_filter());
    }
    Chooser { search, list, selected: selected_row }
}

/// Scroll the chosen row into view once the list is on screen.
pub fn reveal(row: &adw::ActionRow) {
    let row = row.clone();
    glib::idle_add_local_once(move || {
        if let Some(sw) = row.ancestor(gtk::ScrolledWindow::static_type()).and_downcast::<gtk::ScrolledWindow>() {
            if let Some(p) = row.compute_point(&sw, &gtk::graphene::Point::new(0.0, 0.0)) {
                let adj = sw.vadjustment();
                adj.set_value((adj.value() + p.y() as f64 - sw.height() as f64 / 3.0).max(0.0));
            }
        }
    });
}

pub fn language_list(selected: &str, on_select: impl Fn(&'static Language) + 'static) -> Chooser {
    // Subtitle: the language's name in the current language when iso-codes
    // has it ("Czech" → "čeština"), else in English; both are searchable.
    let names = crate::i18n::load_catalog("iso_639-2", &crate::i18n::language()).unwrap_or_default();
    let items = LANGUAGES
        .iter()
        .map(|l| {
            let sub = names.get(l.english).filter(|n| !n.is_empty()).cloned().unwrap_or_else(|| l.english.to_string());
            (l.locale.to_string(), l.native.to_string(), sub, None, format!("{} {}", l.english, l.welcome))
        })
        .collect();
    choice_list(items, selected, tr("Search languages"), Rc::new(move |id: &str| {
        if let Some(l) = language(id) {
            on_select(l);
        }
    }))
}

pub struct Country {
    pub code: String,
    pub name: String,
    pub flag: String,
}

/// Countries from iso-codes, sorted by name.
pub fn countries() -> Vec<Country> {
    let text = std::fs::read_to_string("/usr/share/iso-codes/json/iso_3166-1.json").unwrap_or_default();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
    // Names in the current language, from iso-codes' own translations.
    let names = crate::i18n::load_catalog("iso_3166-1", &crate::i18n::language()).unwrap_or_default();
    let mut v: Vec<Country> = json["3166-1"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|c| {
                    let code = c["alpha_2"].as_str()?.to_string();
                    let english = c["common_name"].as_str().or_else(|| c["name"].as_str())?;
                    let name = names.get(english).filter(|n| !n.is_empty()).cloned().unwrap_or_else(|| english.to_string());
                    let flag = c["flag"].as_str().unwrap_or("").to_string();
                    Some(Country { code, name, flag })
                })
                .collect()
        })
        .unwrap_or_default();
    // Collate in the session language (Č after C in Czech, not after Z).
    v.sort_by_cached_key(|c| glib::CollationKey::from(&c.name));
    v
}

pub fn country_list(selected: &str, on_select: impl Fn(&str) + 'static) -> Chooser {
    let items = countries().into_iter().map(|c| (c.code.clone(), c.name, String::new(), Some(c.flag), c.code)).collect();
    choice_list(items, selected, tr("Search countries"), Rc::new(move |id: &str| on_select(id)))
}

/// Language of the running system, if it is one MAN offers.
pub fn system_language() -> &'static Language {
    let from_file = std::fs::read_to_string("/etc/default/locale")
        .ok()
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("LANG=").map(|v| v.trim_matches('"').to_string())));
    from_file
        .or_else(|| std::env::var("LANG").ok())
        .and_then(|l| language(&l))
        .or_else(|| language(DEFAULT_LOCALE))
        .unwrap()
}
