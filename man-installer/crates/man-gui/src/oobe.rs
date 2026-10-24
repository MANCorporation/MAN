//! "MAN Setup": first-start setup (OOBE). Runs fullscreen in a minimal mutter
//! session as the locked man-oobe account; the system helper applies the
//! result. Every look change is applied live with a crossfade.
use crate::i18n::{tr, trf, tr_dyn, n_};
use crate::backend;
use crate::ui::*;
use adw::prelude::*;
use gtk::{gdk, gio, glib};
use man_common::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const STEPS: [&str; 9] = ["language", "country", "licenses", "network", "account", "customize", "privacy", "encryption", "preparing"];
const MAN_WALLPAPER: &str = "/usr/share/backgrounds/man/wallpaper.jpg";
/// Wallpaper thumbnails made at package installation (`man-oobe --generate-thumbnails`).
const SYSTEM_THUMBS: &str = "/var/cache/man-oobe/thumbnails";
const THUMB_SIZE: i32 = 272;
/// Thumbnails decoded at the same time: big JPEG XL wallpapers are slow.
const THUMB_WORKERS: usize = 3;

const CSS: &str = r#"
.oobe-backdrop { background: #1d1d20; }
.oobe-dim { background: linear-gradient(to bottom, alpha(black, 0.10), alpha(black, 0.35)); }
.oobe-card {
  background: var(--window-bg-color);
  color: var(--window-fg-color);
  border-radius: 28px;
  box-shadow: 0 18px 48px alpha(black, 0.38), 0 0 0 1px alpha(black, 0.06);
}
.oobe-title { font-weight: 800; font-size: 2em; }
.oobe-hero { -gtk-icon-size: 128px; }
.oobe-symbol { -gtk-icon-size: 72px; color: var(--accent-color); }
.step-dot {
  min-width: 8px; min-height: 8px; border-radius: 99px;
  background: alpha(currentColor, 0.18);
  transition: min-width 300ms cubic-bezier(0.25, 0.46, 0.45, 0.94), background 300ms ease;
}
.step-dot.done { background: alpha(var(--accent-bg-color), 0.55); }
.step-dot.active { min-width: 28px; background: var(--accent-bg-color); }
.theme-tile { padding: 6px; border-radius: 16px; background: none; box-shadow: none; }
.theme-tile:checked { box-shadow: inset 0 0 0 3px var(--accent-bg-color); background: none; }
.theme-preview { border-radius: 10px; min-width: 176px; min-height: 110px; }
.theme-preview.light { background: linear-gradient(to bottom, #fafafb 0%, #fafafb 22%, #ebebed 22%); box-shadow: 0 0 0 1px alpha(black, 0.12); }
.theme-preview.dark { background: linear-gradient(to bottom, #36363a 0%, #36363a 22%, #222226 22%); box-shadow: 0 0 0 1px alpha(black, 0.30); }
.theme-preview .bar { min-height: 10px; min-width: 64px; border-radius: 99px; margin: 12px; background: var(--accent-bg-color); }
.accent-swatch {
  min-width: 34px; min-height: 34px; padding: 0; border-radius: 99px;
  transition: box-shadow 200ms ease, transform 200ms ease;
}
.accent-swatch:hover { transform: scale(1.08); }
.accent-swatch:checked { box-shadow: 0 0 0 3px var(--window-bg-color), 0 0 0 6px alpha(currentColor, 0.9); }
.accent-swatch image { color: white; }
.wallpaper-thumb { border-radius: 12px; background: alpha(currentColor, 0.08); }
flowboxchild.wallpaper-child { padding: 4px; border-radius: 16px; background: none; }
flowboxchild.wallpaper-child:selected { box-shadow: inset 0 0 0 3px var(--accent-bg-color); background: none; }
.license-text { font-size: 0.95em; }
.prepare-spinner { -gtk-icon-size: 64px; }
.done-icon { -gtk-icon-size: 96px; color: var(--success-color); }
"#;

#[derive(Clone)]
struct Wallpaper {
    name: String,
    light: String,
    dark: String,
}

#[derive(Clone, PartialEq)]
enum Encryption {
    Encrypted,
    Available,
    Unavailable(String),
}

#[derive(Default)]
struct State {
    language: String,
    country: String,
    country_chosen: bool,
    accepted: bool,
    user: User,
    password2: String,
    username_edited: bool,
    hostname: String,
    hostname_edited: bool,
    set_root_password: bool,
    root_password: String,
    root_password2: String,
    dark: bool,
    accent: String,
    wallpaper: usize,
    location: bool,
    automatic_timezone: bool,
    encrypt: bool,
    passphrase: String,
    passphrase2: String,
    finished: bool,
}

struct Ctx {
    window: adw::ApplicationWindow,
    nav: adw::NavigationView,
    toasts: adw::ToastOverlay,
    root: gtk::Overlay,
    fade: gtk::Picture,
    backdrop: gtk::Stack,
    dots: Vec<gtk::Box>,
    accent_css: gtk::CssProvider,
    wallpapers: Vec<Wallpaper>,
    encryption: Encryption,
    homes: Vec<String>,
    state: RefCell<State>,
    backdrop_serial: Cell<u32>,
    thumbs: RefCell<Vec<Option<gdk::Texture>>>,
    thumb_pictures: RefCell<Vec<Option<gtk::Picture>>>,
}

type C = Rc<Ctx>;

// ─────────────────────────── Environment ───────────────────────────

fn read_wallpapers() -> Vec<Wallpaper> {
    let tag = |text: &str, t: &str| -> Option<String> {
        let open = format!("<{t}>");
        let start = text.find(&open)? + open.len();
        let end = text[start..].find(&format!("</{t}>"))? + start;
        Some(text[start..end].trim().to_string())
    };
    let mut out = vec![];
    let mut files: Vec<_> = std::fs::read_dir("/usr/share/gnome-background-properties")
        .map(|r| r.filter_map(|e| e.ok()).map(|e| e.path()).collect())
        .unwrap_or_default();
    files.sort();
    for f in files {
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        for block in text.split("<wallpaper").skip(1) {
            let (Some(name), Some(light)) = (tag(block, "name"), tag(block, "filename")) else { continue };
            if !std::path::Path::new(&light).is_file() || light.contains("symbolic") || light.contains("/vnc-") {
                continue;
            }
            let dark = tag(block, "filename-dark").filter(|d| std::path::Path::new(d).is_file()).unwrap_or_else(|| light.clone());
            if out.iter().any(|w: &Wallpaper| w.light == light) {
                continue;
            }
            out.push(Wallpaper { name, light, dark });
        }
    }
    // The MAN wallpaper first: it is the MAN default.
    if let Some(i) = out.iter().position(|w| w.light == MAN_WALLPAPER) {
        let w = out.remove(i);
        out.insert(0, w);
    } else if std::path::Path::new(MAN_WALLPAPER).is_file() {
        out.insert(0, Wallpaper { name: "MAN".into(), light: MAN_WALLPAPER.into(), dark: MAN_WALLPAPER.into() });
    }
    out
}

fn command(cmd: &str, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new(cmd).args(args).env("LC_ALL", "C.UTF-8").output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn detect_encryption() -> Encryption {
    let source = |mp: &str| command("findmnt", &["-n", "-o", "SOURCE", "--mountpoint", mp]).map(|s| s.split('[').next().unwrap_or("").to_string());
    let Some(root) = source("/") else { return Encryption::Unavailable(tr("The system disk was not found.").into()) };
    if root.starts_with("/dev/mapper/") || root.starts_with("/dev/dm-") {
        return Encryption::Encrypted;
    }
    if std::path::Path::new(ENCRYPT_PENDING).exists() {
        return Encryption::Encrypted;
    }
    if command("findmnt", &["-n", "-o", "FSTYPE", "--mountpoint", "/"]).as_deref() != Some("btrfs") {
        return Encryption::Unavailable(tr("Encryption needs the Btrfs layout created by the MAN installer.").into());
    }
    let label = source("/boot").and_then(|b| command("lsblk", &["-n", "-o", "LABEL", &b]));
    if label.as_deref() != Some("MAN_BOOT") {
        return Encryption::Unavailable(tr("This installation has no separate boot partition, which encryption needs. Reinstall MAN with “Erase Disk” to use encryption.").into());
    }
    Encryption::Available
}

/// Folders in /home that belong to no account (kept by a reinstall).
fn orphan_homes() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir("/home")
        .map(|r| {
            r.filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n != "lost+found" && command("getent", &["passwd", n]).is_none() && valid_username(n))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

fn thumb_name(file: &str) -> String {
    // FNV-1a of path and modification time: a changed wallpaper gets a new thumbnail.
    let mtime = std::fs::metadata(file).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
    let mut h: u64 = 0xcbf29ce484222325;
    for b in format!("{file}:{mtime}").bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}.png")
}

fn user_thumbs() -> std::path::PathBuf {
    glib::user_cache_dir().join("man-oobe/thumbnails")
}

/// Thumbnail pixbuf: from a cache, or decoded and stored in the user cache.
fn thumb_pixbuf(file: &str) -> Option<gtk::gdk_pixbuf::Pixbuf> {
    let name = thumb_name(file);
    for dir in [std::path::PathBuf::from(SYSTEM_THUMBS), user_thumbs()] {
        if let Ok(pb) = gtk::gdk_pixbuf::Pixbuf::from_file(dir.join(&name)) {
            return Some(pb);
        }
    }
    let pb = gtk::gdk_pixbuf::Pixbuf::from_file_at_scale(file, THUMB_SIZE, THUMB_SIZE, true).ok()?;
    let dir = user_thumbs();
    if std::fs::create_dir_all(&dir).is_ok() {
        let _ = pb.savev(dir.join(&name), "png", &[]);
    }
    Some(pb)
}

/// `man-oobe --generate-thumbnails`: fill the system cache (package installation).
pub fn generate_thumbnails() -> i32 {
    let _ = std::fs::create_dir_all(SYSTEM_THUMBS);
    let mut made = 0;
    for w in read_wallpapers() {
        let name = thumb_name(&w.light);
        let path = std::path::Path::new(SYSTEM_THUMBS).join(&name);
        if path.exists() {
            continue;
        }
        if let Ok(pb) = gtk::gdk_pixbuf::Pixbuf::from_file_at_scale(&w.light, THUMB_SIZE, THUMB_SIZE, true) {
            if pb.savev(&path, "png", &[]).is_ok() {
                made += 1;
            }
        }
    }
    println!("man-oobe: {made} wallpaper thumbnails created in {SYSTEM_THUMBS}");
    0
}

fn pixbuf_texture(pb: (glib::Bytes, i32, i32, usize, bool)) -> gdk::Texture {
    let (bytes, w, h, stride, alpha) = pb;
    let format = if alpha { gdk::MemoryFormat::R8g8b8a8 } else { gdk::MemoryFormat::R8g8b8 };
    gdk::MemoryTexture::new(w, h, format, &bytes, stride).upcast()
}

fn pixbuf_parts(pb: gtk::gdk_pixbuf::Pixbuf) -> (glib::Bytes, i32, i32, usize, bool) {
    (pb.read_pixel_bytes(), pb.width(), pb.height(), pb.rowstride() as usize, pb.has_alpha())
}

/// Decode an image scaled to fit `w`×`h` on a worker thread.
async fn load_texture(path: String, w: i32, h: i32) -> Option<gdk::Texture> {
    let decoded = gio::spawn_blocking(move || gtk::gdk_pixbuf::Pixbuf::from_file_at_scale(&path, w, h, true).ok().map(pixbuf_parts)).await.ok().flatten()?;
    Some(pixbuf_texture(decoded))
}

// ─────────────────────────── Live appearance ───────────────────────────

impl Ctx {
    fn toast(&self, t: &str) {
        self.toasts.add_toast(adw::Toast::new(t));
    }

    /// Freeze the current frame on top, apply the change underneath, fade
    /// the frozen frame out: a smooth crossfade of the whole screen.
    fn crossfade(&self, change: impl FnOnce()) {
        let still = gtk::WidgetPaintable::new(Some(&self.root)).current_image();
        self.fade.set_paintable(Some(&still));
        self.fade.set_opacity(1.0);
        self.fade.set_visible(true);
        change();
        let target = adw::PropertyAnimationTarget::new(&self.fade, "opacity");
        let anim = adw::TimedAnimation::new(&self.fade, 1.0, 0.0, 520, target);
        anim.set_easing(adw::Easing::EaseInOutCubic);
        let fade = self.fade.clone();
        anim.connect_done(move |_| {
            fade.set_visible(false);
            fade.set_paintable(gdk::Paintable::NONE);
        });
        anim.play();
    }

    fn apply_scheme(&self) {
        let dark = self.state.borrow().dark;
        adw::StyleManager::default().set_color_scheme(if dark { adw::ColorScheme::ForceDark } else { adw::ColorScheme::ForceLight });
    }

    fn apply_accent(&self) {
        let accent = self.state.borrow().accent.clone();
        let hex = ACCENTS.iter().find(|a| a.0 == accent).map(|a| a.2).unwrap_or("#3584e4");
        self.accent_css.load_from_string(&format!(":root {{ --accent-bg-color: {hex}; }}"));
    }

    fn current_wallpaper_file(&self) -> Option<String> {
        let s = self.state.borrow();
        self.wallpapers.get(s.wallpaper).map(|w| if s.dark { w.dark.clone() } else { w.light.clone() })
    }

    /// Load the wallpaper for the current choice and crossfade to it.
    fn refresh_backdrop(self: &Rc<Self>, animate: bool) {
        let Some(file) = self.current_wallpaper_file() else { return };
        let serial = self.backdrop_serial.get() + 1;
        self.backdrop_serial.set(serial);
        let ctx = self.clone();
        glib::spawn_future_local(async move {
            let Some(texture) = load_texture(file, 1920, 1920).await else { return };
            if ctx.backdrop_serial.get() != serial {
                return; // a newer choice was made meanwhile
            }
            let pic = gtk::Picture::builder().paintable(&texture).content_fit(gtk::ContentFit::Cover).can_shrink(true).build();
            let name = format!("bg{serial}");
            ctx.backdrop.add_named(&pic, Some(&name));
            let old: Vec<gtk::Widget> = {
                let mut v = vec![];
                let mut c = ctx.backdrop.first_child();
                while let Some(w) = c {
                    c = w.next_sibling();
                    if w != pic.clone().upcast::<gtk::Widget>() {
                        v.push(w);
                    }
                }
                v
            };
            ctx.backdrop.set_transition_duration(if animate { 650 } else { 0 });
            ctx.backdrop.set_visible_child_name(&name);
            let backdrop = ctx.backdrop.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(800), move || {
                for w in old {
                    backdrop.remove(&w);
                }
            });
        });
    }

    /// Load all wallpaper thumbnails in the background, a few at a time, so
    /// they are ready by the time the Customize page is reached.
    fn preload_thumbnails(self: &Rc<Self>) {
        let next = Rc::new(Cell::new(0usize));
        for _ in 0..THUMB_WORKERS {
            let (ctx, next) = (self.clone(), next.clone());
            glib::spawn_future_local(async move {
                loop {
                    let i = next.get();
                    if i >= ctx.wallpapers.len() {
                        break;
                    }
                    next.set(i + 1);
                    let file = ctx.wallpapers[i].light.clone();
                    let Ok(Some(parts)) = gio::spawn_blocking(move || thumb_pixbuf(&file).map(pixbuf_parts)).await else { continue };
                    let texture = pixbuf_texture(parts);
                    ctx.thumbs.borrow_mut()[i] = Some(texture.clone());
                    let pic = ctx.thumb_pictures.borrow().get(i).cloned().flatten();
                    if let Some(pic) = pic {
                        show_thumb(&pic, &texture);
                    }
                }
            });
        }
    }

    fn set_dark(self: &Rc<Self>, dark: bool) {
        if self.state.borrow().dark == dark {
            return;
        }
        self.crossfade(|| {
            self.state.borrow_mut().dark = dark;
            self.apply_scheme();
        });
        self.refresh_backdrop(true);
    }

    fn set_accent(&self, accent: &str) {
        if self.state.borrow().accent == accent {
            return;
        }
        self.crossfade(|| {
            self.state.borrow_mut().accent = accent.to_string();
            self.apply_accent();
        });
    }

    fn set_wallpaper(self: &Rc<Self>, i: usize) {
        if self.state.borrow().wallpaper == i {
            return;
        }
        self.state.borrow_mut().wallpaper = i;
        self.refresh_backdrop(true);
    }

    fn update_dots(&self) {
        let tag = self.nav.visible_page().and_then(|p| p.tag()).map(|t| t.to_string()).unwrap_or_default();
        let current = STEPS.iter().position(|s| *s == tag).unwrap_or(0);
        for (i, d) in self.dots.iter().enumerate() {
            d.remove_css_class("active");
            d.remove_css_class("done");
            if i == current {
                d.add_css_class("active");
            } else if i < current {
                d.add_css_class("done");
            }
        }
    }
}

/// Put a thumbnail into its tile with a short fade-in.
fn show_thumb(pic: &gtk::Picture, texture: &gdk::Texture) {
    pic.set_paintable(Some(texture));
    let anim = adw::TimedAnimation::new(pic, 0.0, 1.0, 260, adw::PropertyAnimationTarget::new(pic, "opacity"));
    anim.play();
}

// ─────────────────────────── Page scaffolding ───────────────────────────

fn heading(icon_name: Option<&str>, title: &str, description: &str) -> gtk::Box {
    let b = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(10).margin_bottom(18).build();
    if let Some(i) = icon_name {
        let img = gtk::Image::from_icon_name(i);
        img.add_css_class("oobe-symbol");
        b.append(&img);
    }
    b.append(&label(title, &["oobe-title"]));
    if !description.is_empty() {
        b.append(&label(description, &["dim-label"]));
    }
    b
}

/// A setup page: scrollable, clamped content and a bottom action bar.
fn setup_page(tag: &str, title: &str, content: &impl IsA<gtk::Widget>, bottom: &impl IsA<gtk::Widget>) -> adw::NavigationPage {
    let tv = adw::ToolbarView::new();
    let header = adw::HeaderBar::builder().show_end_title_buttons(false).show_start_title_buttons(false).show_title(false).build();
    tv.add_top_bar(&header);
    let clamp = adw::Clamp::builder().maximum_size(560).child(content).margin_start(24).margin_end(24).margin_bottom(12).build();
    let scroll = gtk::ScrolledWindow::builder().hscrollbar_policy(gtk::PolicyType::Never).vexpand(true).child(&clamp).build();
    tv.set_content(Some(&scroll));
    let bar = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(6).margin_top(10).margin_bottom(22).margin_start(24).margin_end(24).build();
    bar.append(bottom);
    tv.add_bottom_bar(&bar);
    adw::NavigationPage::builder().title(title).tag(tag).child(&tv).build()
}

fn next_bar(next: &gtk::Button, hint: &gtk::Label) -> gtk::Box {
    let b = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(6).build();
    b.append(hint);
    b.append(next);
    b
}

fn group(title: &str, description: &str) -> adw::PreferencesGroup {
    let g = adw::PreferencesGroup::builder().title(title).build();
    if !description.is_empty() {
        g.set_description(Some(description));
    }
    g
}

fn listbox() -> gtk::ListBox {
    let l = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).build();
    l.add_css_class("boxed-list");
    l
}

fn vbox(spacing: i32) -> gtk::Box {
    gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(spacing).build()
}

// ─────────────────────────── Language & country ───────────────────────────

/// First page: the globe over "Welcome" in every language, and the list.
fn language_page(ctx: &C) -> adw::NavigationPage {
    let current = man_common::language(&ctx.state.borrow().language).unwrap_or(&man_common::LANGUAGES[0]);
    let hero = crate::langui::hero(current.welcome);
    let title = hero.title.clone();
    let next = pill(tr("_Next"), Some("suggested-action"));
    let search_slot: Rc<RefCell<Option<gtk::SearchEntry>>> = Rc::default();
    let chooser = {
        let ctx = ctx.clone();
        let next = next.clone();
        let search_slot = search_slot.clone();
        crate::langui::language_list(current.locale, move |l| {
            let mut s = ctx.state.borrow_mut();
            s.language = l.locale.to_string();
            if !s.country_chosen && !l.country.is_empty() {
                s.country = l.country.to_string();
            }
            drop(s);
            crate::langui::set_hero_title(&title, l.welcome);
            // Everything from here on is shown in the chosen language.
            crate::i18n::set_language(l.locale);
            next.set_label(tr("_Next"));
            if let Some(search) = search_slot.borrow().as_ref() {
                search.set_placeholder_text(Some(tr("Search languages")));
            }
            ctx.window.set_title(Some(tr("MAN Setup")));
        })
    };
    *search_slot.borrow_mut() = Some(chooser.search.clone());
    {
        let ctx = ctx.clone();
        next.connect_clicked(move |_| ctx.nav.push(&country_page(&ctx)));
    }
    let (page, content) = chooser_page("language", tr("Language"), &hero.widget, &chooser, &next);

    // Entrance: fade in while gliding up.
    content.set_opacity(0.0);
    let c2 = content.clone();
    page.connect_shown(move |_| {
        if c2.opacity() > 0.0 {
            return;
        }
        let fade = adw::TimedAnimation::new(&c2, 0.0, 1.0, 700, adw::PropertyAnimationTarget::new(&c2, "opacity"));
        fade.set_easing(adw::Easing::EaseOutCubic);
        fade.play();
    });
    let next2 = next.clone();
    let chosen = chooser.selected.clone();
    page.connect_shown(move |_| {
        next2.grab_focus();
        if let Some(r) = &chosen {
            crate::langui::reveal(r);
        }
    });
    page
}

/// A page with a fixed top (heading and search) over a scrolling list.
fn chooser_page(tag: &str, title: &str, top: &impl IsA<gtk::Widget>, chooser: &crate::langui::Chooser, next: &gtk::Button) -> (adw::NavigationPage, gtk::Box) {
    let head = vbox(10);
    head.append(top);
    head.append(&adw::Clamp::builder().maximum_size(560).child(&chooser.search).margin_start(24).margin_end(24).margin_bottom(6).build());
    let list_clamp = adw::Clamp::builder().maximum_size(560).child(&chooser.list).margin_start(24).margin_end(24).margin_bottom(12).margin_top(4).build();
    let scroll = gtk::ScrolledWindow::builder().hscrollbar_policy(gtk::PolicyType::Never).vexpand(true).child(&list_clamp).build();
    let content = vbox(0);
    content.append(&head);
    content.append(&scroll);

    let tv = adw::ToolbarView::new();
    tv.add_top_bar(&adw::HeaderBar::builder().show_end_title_buttons(false).show_start_title_buttons(false).show_title(false).build());
    tv.set_content(Some(&content));
    let bar = gtk::Box::builder().orientation(gtk::Orientation::Vertical).margin_top(10).margin_bottom(22).margin_start(24).margin_end(24).build();
    bar.append(next);
    tv.add_bottom_bar(&bar);
    (adw::NavigationPage::builder().title(title).tag(tag).child(&tv).build(), content)
}

fn country_page(ctx: &C) -> adw::NavigationPage {
    let top = heading(Some("mark-location-symbolic"), tr("Where Are You?"), tr("Your country or region sets the date, time and number formats."));
    top.set_margin_start(24);
    top.set_margin_end(24);
    top.set_margin_bottom(4);
    let selected = ctx.state.borrow().country.clone();
    let chooser = {
        let ctx = ctx.clone();
        crate::langui::country_list(&selected, move |code| {
            let mut s = ctx.state.borrow_mut();
            s.country = code.to_string();
            s.country_chosen = true;
        })
    };
    let next = pill(tr("_Next"), Some("suggested-action"));
    {
        let ctx = ctx.clone();
        next.connect_clicked(move |_| ctx.nav.push(&licenses_page(&ctx)));
    }
    let (page, _) = chooser_page("country", tr("Country"), &top, &chooser, &next);
    let chosen = chooser.selected.clone();
    page.connect_shown(move |_| {
        if let Some(r) = &chosen {
            crate::langui::reveal(r);
        }
    });
    page
}

// ─────────────────────────── Licenses ───────────────────────────

const LICENSE_SUMMARY: &str = "MAN is built from thousands of free and open-source components made by the Linux, GNU and GNOME projects and many others. Each of them keeps its own license, which gives you the rights it grants, such as using, studying, changing and sharing that component. The most common licenses are listed below; the full license of every component is in /usr/share/doc.\n\nMAN's own programs (MAN Installer, MAN Setup, Snapshots and MAN Update) and the MAN artwork are © MAN Corporation, all rights reserved. You may use them as part of MAN, but not copy, change or redistribute them on their own without permission.\n\nMAN comes with ABSOLUTELY NO WARRANTY, to the extent permitted by law.";

const LICENSES: &[(&str, &str, &str)] = &[
    (n_("GNU General Public License, version 2"), n_("The Linux kernel and many system tools"), "GPL-2"),
    (n_("GNU General Public License, version 3"), n_("Many GNU and GNOME programs"), "GPL-3"),
    (n_("GNU Lesser General Public License, version 2.1"), n_("GTK, GLib and many libraries"), "LGPL-2.1"),
    (n_("GNU Lesser General Public License, version 3"), n_("Libraries"), "LGPL-3"),
    (n_("Mozilla Public License 2.0"), n_("Programs and libraries from Mozilla and others"), "MPL-2.0"),
    (n_("Apache License 2.0"), n_("Libraries and tools"), "Apache-2.0"),
];

fn licenses_page(ctx: &C) -> adw::NavigationPage {
    let content = vbox(18);
    content.append(&heading(Some("text-x-generic-symbolic"), tr("License Agreement"), tr("Please read and accept the terms under which MAN is provided.")));
    let summary = gtk::Label::builder().label(LICENSE_SUMMARY).wrap(true).xalign(0.0).build();
    summary.add_css_class("license-text");
    let card = gtk::Box::builder().margin_top(4).build();
    card.add_css_class("card");
    summary.set_margin_top(16);
    summary.set_margin_bottom(16);
    summary.set_margin_start(16);
    summary.set_margin_end(16);
    card.append(&summary);
    content.append(&card);
    let g = group(tr("Licenses"), "");
    for (title, sub, file) in LICENSES {
        let row = adw::ActionRow::builder().title(tr(title)).subtitle(tr(sub)).activatable(true).build();
        row.add_suffix(&icon("go-next-symbolic"));
        let path = format!("/usr/share/common-licenses/{file}");
        let ctx2 = ctx.clone();
        let t = title.to_string();
        row.connect_activated(move |_| {
            let text = std::fs::read_to_string(&path).unwrap_or_else(|_| trf("The license text is not available ({path}).", &[("path", &path)]));
            details_dialog(&ctx2.window, &t, &text);
        });
        g.add(&row);
    }
    content.append(&g);
    let accept = gtk::CheckButton::builder().label(tr("I have read and accept the license terms")).halign(gtk::Align::Center).margin_top(6).build();
    accept.set_active(ctx.state.borrow().accepted);
    content.append(&accept);

    let next = pill(tr("_Next"), Some("suggested-action"));
    next.set_sensitive(ctx.state.borrow().accepted);
    let hint = label("", &["dim-label", "caption"]);
    hint.set_visible(false);
    {
        let ctx = ctx.clone();
        let next = next.clone();
        accept.connect_toggled(move |c| {
            ctx.state.borrow_mut().accepted = c.is_active();
            next.set_sensitive(c.is_active());
        });
    }
    {
        let ctx = ctx.clone();
        next.connect_clicked(move |_| ctx.nav.push(&network_page(&ctx)));
    }
    setup_page("licenses", tr("Licenses"), &content, &next_bar(&next, &hint))
}

// ─────────────────────────── Network ───────────────────────────

#[derive(Clone, Debug)]
struct WifiNet {
    ssid: String,
    signal: u32,
    secure: bool,
    in_use: bool,
}

/// Split one `nmcli -t` line, honoring `\:` escapes.
fn nm_fields(line: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(n) = chars.next() {
                    out.last_mut().unwrap().push(n);
                }
            }
            ':' => out.push(String::new()),
            c => out.last_mut().unwrap().push(c),
        }
    }
    out
}

struct NetStatus {
    connected: bool,
    description: String,
    wifi_device: bool,
}

fn net_status() -> NetStatus {
    let devices = command("nmcli", &["-t", "-f", "DEVICE,TYPE,STATE,CONNECTION", "device"]).unwrap_or_default();
    let mut connected = None;
    let mut wifi_device = false;
    for l in devices.lines() {
        let f = nm_fields(l);
        if f.len() < 4 {
            continue;
        }
        if f[1] == "wifi" {
            wifi_device = true;
        }
        if f[2] == "connected" && (f[1] == "ethernet" || f[1] == "wifi") && connected.is_none() {
            let kind = if f[1] == "wifi" { "Wi-Fi" } else { tr("Wired") };
            connected = Some(format!("{kind}: {}", f[3]));
        }
    }
    match connected {
        Some(d) => NetStatus { connected: true, description: d, wifi_device },
        None => NetStatus { connected: false, description: tr("Not connected").into(), wifi_device },
    }
}

fn wifi_networks() -> Vec<WifiNet> {
    let out = command("nmcli", &["-t", "-f", "IN-USE,SSID,SIGNAL,SECURITY", "device", "wifi", "list", "--rescan", "auto"]).unwrap_or_default();
    let mut nets: Vec<WifiNet> = vec![];
    for l in out.lines() {
        let f = nm_fields(l);
        if f.len() < 4 || f[1].is_empty() {
            continue;
        }
        let net = WifiNet { in_use: f[0] == "*", ssid: f[1].clone(), signal: f[2].parse().unwrap_or(0), secure: !f[3].is_empty() && f[3] != "--" };
        match nets.iter_mut().find(|n| n.ssid == net.ssid) {
            Some(n) if n.signal >= net.signal => {}
            Some(n) => *n = net,
            None => nets.push(net),
        }
    }
    nets.sort_by(|a, b| b.in_use.cmp(&a.in_use).then(b.signal.cmp(&a.signal)));
    nets
}

fn signal_icon(s: u32) -> &'static str {
    match s {
        0..=24 => "network-wireless-signal-weak-symbolic",
        25..=49 => "network-wireless-signal-ok-symbolic",
        50..=74 => "network-wireless-signal-good-symbolic",
        _ => "network-wireless-signal-excellent-symbolic",
    }
}

fn network_page(ctx: &C) -> adw::NavigationPage {
    let content = vbox(18);
    content.append(&heading(Some("network-wireless-symbolic"), tr("Connect to the Internet"), tr("Connecting lets MAN set the time and keep your software up to date. You can also do this later.")));
    let status_group = group("", "");
    let status = adw::ActionRow::builder().title(tr("Checking the connection…")).build();
    let status_icon = icon("network-offline-symbolic");
    status.add_prefix(&status_icon);
    status_group.add(&status);
    content.append(&status_group);

    let wifi_group = group(tr("Wi-Fi Networks"), "");
    let spinner = adw::Spinner::builder().visible(false).build();
    let refresh = gtk::Button::builder().icon_name("view-refresh-symbolic").tooltip_text(tr("Search Again")).valign(gtk::Align::Center).build();
    refresh.add_css_class("flat");
    let suffix = gtk::Box::builder().spacing(6).build();
    suffix.append(&spinner);
    suffix.append(&refresh);
    wifi_group.set_header_suffix(Some(&suffix));
    let list = listbox();
    wifi_group.add(&list);
    wifi_group.set_visible(false);
    content.append(&wifi_group);

    let next = pill(tr("_Skip"), Some("suggested-action"));
    let hint = label(tr("You can connect later in Settings."), &["dim-label", "caption"]);
    let connected = Rc::new(Cell::new(false));

    let update_status: Rc<dyn Fn()> = {
        let (status, status_icon, next, hint, wifi_group, connected) = (status.clone(), status_icon.clone(), next.clone(), hint.clone(), wifi_group.clone(), connected.clone());
        Rc::new(move || {
            let (status, status_icon, next, hint, wifi_group, connected) = (status.clone(), status_icon.clone(), next.clone(), hint.clone(), wifi_group.clone(), connected.clone());
            glib::spawn_future_local(async move {
                let Ok(st) = gio::spawn_blocking(net_status).await else { return };
                status.set_title(if st.connected { tr("Connected") } else { tr("Not Connected") });
                status.set_subtitle(&glib::markup_escape_text(&st.description));
                status_icon.set_icon_name(Some(if st.connected { "network-transmit-receive-symbolic" } else { "network-offline-symbolic" }));
                wifi_group.set_visible(st.wifi_device);
                if st.connected != connected.get() || next.label().is_none_or(|l| l == "_Skip") == st.connected {
                    next.set_label(if st.connected { tr("_Next") } else { tr("_Skip") });
                    hint.set_visible(!st.connected);
                }
                connected.set(st.connected);
            });
        })
    };

    let fill_wifi: Rc<RefCell<Box<dyn Fn()>>> = Rc::new(RefCell::new(Box::new(|| {})));
    {
        let ctx = ctx.clone();
        let (list, spinner, refresh, us) = (list.clone(), spinner.clone(), refresh.clone(), update_status.clone());
        let fill_self = fill_wifi.clone();
        *fill_wifi.borrow_mut() = Box::new(move || {
            let (ctx, list, spinner, refresh, us, fill_self) = (ctx.clone(), list.clone(), spinner.clone(), refresh.clone(), us.clone(), fill_self.clone());
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
                    let (ctx, us, fill) = (ctx.clone(), us.clone(), fill_self.clone());
                    row.connect_activated(move |row| connect_wifi(&ctx, row, &n, us.clone(), fill.clone()));
                    list.append(&row);
                }
            });
        });
    }
    {
        let fw = fill_wifi.clone();
        refresh.connect_clicked(move |_| (fw.borrow())());
    }
    {
        let ctx = ctx.clone();
        next.connect_clicked(move |_| ctx.nav.push(&account_page(&ctx)));
    }
    let page = setup_page("network", tr("Internet"), &content, &next_bar(&next, &hint));
    {
        let (us, fw) = (update_status.clone(), fill_wifi.clone());
        page.connect_shown(move |_| {
            us();
            (fw.borrow())();
        });
    }
    // Keep the status current while the page is shown.
    {
        let us = update_status.clone();
        let page_weak = page.downgrade();
        let ctx = ctx.clone();
        glib::timeout_add_seconds_local(3, move || {
            let Some(page) = page_weak.upgrade() else { return glib::ControlFlow::Break };
            if ctx.nav.visible_page().as_ref() == Some(&page) {
                us();
            }
            glib::ControlFlow::Continue
        });
    }
    page
}

fn connect_wifi(ctx: &C, row: &adw::ActionRow, net: &WifiNet, update: Rc<dyn Fn()>, refill: Rc<RefCell<Box<dyn Fn()>>>) {
    let ssid = net.ssid.clone();
    let do_connect = {
        let ctx = ctx.clone();
        let row = row.clone();
        move |password: Option<String>| {
            let (ctx, row, update, refill, ssid) = (ctx.clone(), row.clone(), update.clone(), refill.clone(), ssid.clone());
            row.set_sensitive(false);
            let spinner = adw::Spinner::new();
            row.add_suffix(&spinner);
            glib::spawn_future_local(async move {
                let s2 = ssid.clone();
                let ok = gio::spawn_blocking(move || {
                    let mut args = vec!["device", "wifi", "connect", s2.as_str()];
                    if let Some(p) = password.as_deref() {
                        args.extend(["password", p]);
                    }
                    command("nmcli", &args).is_some()
                })
                .await
                .unwrap_or(false);
                row.remove(&spinner);
                row.set_sensitive(true);
                ctx.toast(&if ok { trf("Connected to “{ssid}”", &[("ssid", &ssid)]) } else { trf("Could not connect to “{ssid}”", &[("ssid", &ssid)]) });
                update();
                (refill.borrow())();
            });
        }
    };
    if !net.secure {
        do_connect(None);
        return;
    }
    let dialog = adw::AlertDialog::new(Some(&trf("Connect to “{ssid}”", &[("ssid", &net.ssid)])), Some(tr("Enter the network password.")));
    let entry = gtk::PasswordEntry::builder().show_peek_icon(true).activates_default(true).build();
    dialog.set_extra_child(Some(&entry));
    dialog.add_response("cancel", tr("_Cancel"));
    dialog.add_response("connect", tr("C_onnect"));
    dialog.set_response_appearance("connect", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("connect"));
    dialog.set_close_response("cancel");
    let window = ctx.window.clone();
    glib::spawn_future_local(async move {
        let e = entry.clone();
        glib::idle_add_local_once(move || {
            e.grab_focus();
        });
        if dialog.choose_future(Some(&window)).await == "connect" {
            do_connect(Some(entry.text().to_string()));
        }
    });
}

// ─────────────────────────── Account ───────────────────────────

fn account_page(ctx: &C) -> adw::NavigationPage {
    let prefs = vbox(18);
    let s = ctx.state.borrow();
    let avatar = adw::Avatar::builder().size(96).show_initials(true).text(s.user.full_name.as_str()).build();
    prefs.append(&avatar);
    prefs.append(&heading(None, tr("Create Your Account"), tr("You will be the administrator of this computer, with full root access through sudo.")));

    let g = group("", "");
    let full = adw::EntryRow::builder().title(tr("Full Name")).text(s.user.full_name.as_str()).build();
    let user = adw::EntryRow::builder().title(tr("Username")).text(s.user.username.as_str()).build();
    let pw = adw::PasswordEntryRow::builder().title(tr("Password")).text(s.user.password.as_str()).build();
    let pw2 = adw::PasswordEntryRow::builder().title(tr("Confirm Password")).text(s.password2.as_str()).build();
    let auto = adw::SwitchRow::builder().title(tr("Log In Automatically")).subtitle(tr("Skip the login screen when the computer starts")).active(s.user.autologin).build();
    for w in [full.upcast_ref::<gtk::Widget>(), user.upcast_ref(), pw.upcast_ref(), pw2.upcast_ref(), auto.upcast_ref()] {
        g.add(w);
    }
    prefs.append(&g);

    if !ctx.homes.is_empty() {
        let hg = group(tr("Existing Home Folders"), tr("Use the same username to keep using a folder kept from a previous installation."));
        for h in &ctx.homes {
            let row = adw::ActionRow::builder().title(format!("/home/{h}")).build();
            row.add_prefix(&icon("user-home-symbolic"));
            let b = gtk::Button::builder().label(tr("Use")).valign(gtk::Align::Center).build();
            let (user, h2) = (user.clone(), h.clone());
            b.connect_clicked(move |_| user.set_text(&h2));
            row.add_suffix(&b);
            row.set_activatable_widget(Some(&b));
            hg.add(&row);
        }
        prefs.append(&hg);
    }

    let adv = group(tr("Advanced"), "");
    let host = adw::EntryRow::builder().title(tr("Computer Name")).text(s.hostname.as_str()).build();
    let root_exp = adw::ExpanderRow::builder()
        .title(tr("Set a Root Password"))
        .subtitle(tr("Allows logging in as root directly. Without it, use sudo with your own password."))
        .show_enable_switch(true)
        .enable_expansion(s.set_root_password)
        .build();
    let rpw = adw::PasswordEntryRow::builder().title(tr("Root Password")).text(s.root_password.as_str()).build();
    let rpw2 = adw::PasswordEntryRow::builder().title(tr("Confirm Root Password")).text(s.root_password2.as_str()).build();
    root_exp.add_row(&rpw);
    root_exp.add_row(&rpw2);
    adv.add(&host);
    adv.add(&root_exp);
    prefs.append(&adv);
    drop(s);

    let next = pill(tr("_Next"), Some("suggested-action"));
    let hint = label("", &["dim-label", "caption"]);
    let validate: Rc<dyn Fn()> = {
        let ctx = ctx.clone();
        let (next, hint, user, pw2, host, rpw2) = (next.clone(), hint.clone(), user.clone(), pw2.clone(), host.clone(), rpw2.clone());
        Rc::new(move || {
            let s = ctx.state.borrow();
            let u = &s.user;
            let taken = !u.username.is_empty() && valid_username(&u.username) && !ctx.homes.contains(&u.username) && command("getent", &["passwd", &u.username]).is_some();
            let bad_user = !u.username.is_empty() && (!valid_username(&u.username) || taken);
            let bad_host = !valid_hostname(&s.hostname);
            set_error(&user, bad_user);
            set_error(&pw2, !s.password2.is_empty() && s.password2 != u.password);
            set_error(&host, bad_host);
            set_error(&rpw2, s.set_root_password && !s.root_password2.is_empty() && s.root_password != s.root_password2);
            let problem = if u.full_name.trim().is_empty() {
                Some(tr("Enter your name."))
            } else if u.username.is_empty() {
                Some(tr("Choose a username."))
            } else if taken {
                Some(tr("This username is already used by the system."))
            } else if bad_user {
                if RESERVED_USERS.contains(&u.username.as_str()) {
                    Some(tr("This username is reserved by the system."))
                } else {
                    Some(tr("Usernames use lowercase letters, digits, - and _, and start with a letter."))
                }
            } else if u.password.is_empty() {
                Some(tr("Choose a password."))
            } else if u.password != s.password2 {
                Some(tr("The passwords do not match."))
            } else if bad_host {
                Some(tr("The computer name may contain letters, digits and dashes."))
            } else if s.set_root_password && (s.root_password.is_empty() || s.root_password != s.root_password2) {
                Some(tr("Enter the root password twice."))
            } else {
                None
            };
            let weak = problem.is_none() && u.password.chars().count() < 8;
            next.set_sensitive(problem.is_none());
            hint.set_label(problem.unwrap_or(if weak { tr("Tip: a password with 8 or more characters is much safer.") } else { "" }));
            hint.set_visible(problem.is_some() || weak);
        })
    };
    {
        let ctx = ctx.clone();
        let (user, avatar, host, v) = (user.clone(), avatar.clone(), host.clone(), validate.clone());
        full.connect_changed(move |e| {
            let text = e.text().to_string();
            avatar.set_text(Some(&text));
            let (edited, host_edited) = {
                let mut s = ctx.state.borrow_mut();
                s.user.full_name = text.clone();
                (s.username_edited, s.hostname_edited)
            };
            if !edited {
                let derived = derive_username(&text);
                user.set_text(&derived);
                ctx.state.borrow_mut().username_edited = false;
                if !host_edited && !derived.is_empty() {
                    host.set_text(&format!("{derived}-pc"));
                    ctx.state.borrow_mut().hostname_edited = false;
                }
            }
            v();
        });
    }
    {
        let ctx = ctx.clone();
        let v = validate.clone();
        user.connect_changed(move |e| {
            let mut s = ctx.state.borrow_mut();
            s.user.username = e.text().into();
            s.username_edited = true;
            drop(s);
            v();
        });
    }
    {
        let ctx = ctx.clone();
        let v = validate.clone();
        host.connect_changed(move |e| {
            let mut s = ctx.state.borrow_mut();
            s.hostname = e.text().into();
            s.hostname_edited = true;
            drop(s);
            v();
        });
    }
    macro_rules! bind_text {
        ($w:expr, $field:ident $(. $sub:ident)?) => {{
            let ctx = ctx.clone();
            let v = validate.clone();
            $w.connect_changed(move |e| {
                ctx.state.borrow_mut().$field$(.$sub)? = e.text().into();
                v();
            });
        }};
    }
    bind_text!(pw, user.password);
    bind_text!(pw2, password2);
    bind_text!(rpw, root_password);
    bind_text!(rpw2, root_password2);
    {
        let ctx = ctx.clone();
        auto.connect_active_notify(move |r| ctx.state.borrow_mut().user.autologin = r.is_active());
    }
    {
        let ctx = ctx.clone();
        let v = validate.clone();
        root_exp.connect_enable_expansion_notify(move |r| {
            ctx.state.borrow_mut().set_root_password = r.enables_expansion();
            v();
        });
    }
    validate();
    {
        let ctx = ctx.clone();
        next.connect_clicked(move |_| ctx.nav.push(&customize_page(&ctx)));
    }
    let page = setup_page("account", tr("Account"), &prefs, &next_bar(&next, &hint));
    page.connect_shown(move |_| {
        full.grab_focus();
    });
    page
}

// ─────────────────────────── Customize ───────────────────────────

fn theme_tile(dark: bool) -> gtk::ToggleButton {
    let preview = gtk::Box::builder().orientation(gtk::Orientation::Vertical).build();
    preview.add_css_class("theme-preview");
    preview.add_css_class(if dark { "dark" } else { "light" });
    let bar = gtk::Box::builder().halign(gtk::Align::Start).valign(gtk::Align::End).vexpand(true).build();
    bar.add_css_class("bar");
    preview.append(&bar);
    let b = vbox(8);
    b.append(&preview);
    b.append(&gtk::Label::new(Some(if dark { tr("Dark") } else { tr("Light") })));
    let t = gtk::ToggleButton::builder().child(&b).build();
    t.add_css_class("theme-tile");
    t
}

fn customize_page(ctx: &C) -> adw::NavigationPage {
    let content = vbox(22);
    content.append(&heading(Some("applications-graphics-symbolic"), tr("Make It Yours"), tr("Pick a style, an accent color and a wallpaper. You see every change right away.")));

    // Style
    let style = group(tr("Style"), "");
    let tiles = gtk::Box::builder().spacing(18).halign(gtk::Align::Center).build();
    let light = theme_tile(false);
    let dark = theme_tile(true);
    dark.set_group(Some(&light));
    if ctx.state.borrow().dark { dark.set_active(true) } else { light.set_active(true) }
    tiles.append(&light);
    tiles.append(&dark);
    style.add(&tiles);
    content.append(&style);
    for (t, is_dark) in [(&light, false), (&dark, true)] {
        let ctx = ctx.clone();
        t.connect_toggled(move |t| {
            if t.is_active() {
                ctx.set_dark(is_dark);
            }
        });
    }

    // Accent color
    let accent = group(tr("Accent Color"), "");
    let swatches = gtk::Box::builder().spacing(12).halign(gtk::Align::Center).margin_top(4).margin_bottom(4).build();
    let accent_css = gtk::CssProvider::new();
    let mut css = String::new();
    let mut first: Option<gtk::ToggleButton> = None;
    for (id, name, hex) in ACCENTS {
        css += &format!(".accent-{id} {{ background: {hex}; color: {hex}; }}\n");
        let name = &tr_dyn(name);
        let b = gtk::ToggleButton::builder().tooltip_text(name.as_str()).valign(gtk::Align::Center).build();
        let mark = gtk::Image::from_icon_name("object-select-symbolic");
        mark.set_opacity(0.0);
        b.set_child(Some(&mark));
        b.add_css_class("accent-swatch");
        b.add_css_class(&format!("accent-{id}"));
        b.update_property(&[gtk::accessible::Property::Label(name.as_str())]);
        if let Some(f) = &first {
            b.set_group(Some(f));
        } else {
            first = Some(b.clone());
        }
        b.set_active(ctx.state.borrow().accent == *id);
        mark.set_opacity(if b.is_active() { 1.0 } else { 0.0 });
        let ctx = ctx.clone();
        let id = id.to_string();
        b.connect_toggled(move |b| {
            let target = if b.is_active() { 1.0 } else { 0.0 };
            let anim = adw::TimedAnimation::new(&mark, mark.opacity(), target, 220, adw::PropertyAnimationTarget::new(&mark, "opacity"));
            anim.play();
            if b.is_active() {
                ctx.set_accent(&id);
            }
        });
        swatches.append(&b);
    }
    accent_css.load_from_string(&css);
    if let Some(display) = gdk::Display::default() {
        #[allow(deprecated)]
        gtk::style_context_add_provider_for_display(&display, &accent_css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }
    accent.add(&swatches);
    content.append(&accent);

    // Wallpaper
    let wp = group(tr("Wallpaper"), "");
    let flow = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .min_children_per_line(3)
        .max_children_per_line(4)
        .column_spacing(6)
        .row_spacing(6)
        .homogeneous(true)
        .build();
    for (i, w) in ctx.wallpapers.iter().enumerate() {
        let pic = gtk::Picture::builder().content_fit(gtk::ContentFit::Cover).height_request(84).width_request(136).build();
        pic.add_css_class("wallpaper-thumb");
        pic.set_overflow(gtk::Overflow::Hidden);
        pic.set_tooltip_text(Some(&w.name));
        let child = gtk::FlowBoxChild::builder().child(&pic).build();
        child.add_css_class("wallpaper-child");
        flow.append(&child);
        if i == ctx.state.borrow().wallpaper {
            flow.select_child(&child);
        }
        let ready = ctx.thumbs.borrow().get(i).cloned().flatten();
        match ready {
            Some(t) => pic.set_paintable(Some(&t)),
            None => {
                if let Some(slot) = ctx.thumb_pictures.borrow_mut().get_mut(i) {
                    *slot = Some(pic.clone());
                }
            }
        }
    }
    {
        let ctx = ctx.clone();
        flow.connect_child_activated(move |_, child| ctx.set_wallpaper(child.index() as usize));
    }
    wp.add(&flow);
    content.append(&wp);

    let next = pill(tr("_Next"), Some("suggested-action"));
    let hint = label("", &["caption"]);
    hint.set_visible(false);
    {
        let ctx = ctx.clone();
        next.connect_clicked(move |_| ctx.nav.push(&privacy_page(&ctx)));
    }
    setup_page("customize", tr("Customize"), &content, &next_bar(&next, &hint))
}

// ─────────────────────────── Privacy ───────────────────────────

fn privacy_page(ctx: &C) -> adw::NavigationPage {
    let content = vbox(18);
    content.append(&heading(Some("location-services-active-symbolic"), tr("Privacy"), tr("Choose which features may use information about you. You can change this at any time in Settings.")));
    let g = group("", "");
    let loc = adw::SwitchRow::builder()
        .title(tr("Location Services"))
        .subtitle(tr("Allows apps such as Maps and Weather to find your approximate location, using nearby Wi-Fi networks."))
        .active(ctx.state.borrow().location)
        .build();
    let tz = adw::SwitchRow::builder()
        .title(tr("Automatic Time Zone"))
        .subtitle(tr("Uses your location to keep the time zone right when you travel."))
        .active(ctx.state.borrow().automatic_timezone)
        .sensitive(ctx.state.borrow().location)
        .build();
    g.add(&loc);
    g.add(&tz);
    content.append(&g);
    {
        let ctx = ctx.clone();
        let tz = tz.clone();
        loc.connect_active_notify(move |r| {
            ctx.state.borrow_mut().location = r.is_active();
            tz.set_sensitive(r.is_active());
        });
    }
    {
        let ctx = ctx.clone();
        tz.connect_active_notify(move |r| ctx.state.borrow_mut().automatic_timezone = r.is_active());
    }
    let next = pill(tr("_Next"), Some("suggested-action"));
    let hint = label("", &["caption"]);
    hint.set_visible(false);
    {
        let ctx = ctx.clone();
        next.connect_clicked(move |_| ctx.nav.push(&encryption_page(&ctx)));
    }
    setup_page("privacy", tr("Privacy"), &content, &next_bar(&next, &hint))
}

// ─────────────────────────── Encryption ───────────────────────────

fn encryption_page(ctx: &C) -> adw::NavigationPage {
    let content = vbox(18);
    let next = pill(tr("_Finish Setup"), Some("suggested-action"));
    let hint = label("", &["dim-label", "caption"]);
    hint.set_visible(false);
    match &ctx.encryption {
        Encryption::Encrypted => {
            content.append(&heading(Some("channel-secure-symbolic"), tr("Your Disk Is Encrypted"), tr("Your data is protected. The passphrase is asked every time the computer starts.")));
        }
        Encryption::Unavailable(why) => {
            content.append(&heading(Some("channel-insecure-symbolic"), tr("Disk Encryption"), tr("Encryption protects your data if the computer is lost or stolen.")));
            let g = group("", "");
            let row = adw::ActionRow::builder().title(tr("Not available on this installation")).subtitle(glib::markup_escape_text(why)).build();
            row.add_prefix(&icon("dialog-information-symbolic"));
            g.add(&row);
            content.append(&g);
        }
        Encryption::Available => {
            content.append(&heading(Some("channel-secure-symbolic"), tr("Disk Encryption"), tr("Encryption protects your files if the computer is lost or stolen. Without the passphrase, nobody can read the disk.")));
            let g = group("", "");
            let exp = adw::ExpanderRow::builder()
                .title(tr("Encrypt the Disk"))
                .subtitle(tr("The passphrase is asked every time the computer starts."))
                .show_enable_switch(true)
                .enable_expansion(ctx.state.borrow().encrypt)
                .expanded(ctx.state.borrow().encrypt)
                .build();
            let p1 = adw::PasswordEntryRow::builder().title(tr("Passphrase")).text(ctx.state.borrow().passphrase.as_str()).build();
            let p2 = adw::PasswordEntryRow::builder().title(tr("Confirm Passphrase")).text(ctx.state.borrow().passphrase2.as_str()).build();
            let note = adw::ActionRow::builder()
                .title(tr("Encryption starts at the next restart"))
                .subtitle(tr("Your disk is then encrypted in the background while you use the computer. If the passphrase is lost, the data cannot be recovered by anyone."))
                .build();
            note.add_prefix(&icon("dialog-warning-symbolic"));
            exp.add_row(&p1);
            exp.add_row(&p2);
            exp.add_row(&note);
            g.add(&exp);
            content.append(&g);
            let validate: Rc<dyn Fn()> = {
                let (ctx, next, hint, p2) = (ctx.clone(), next.clone(), hint.clone(), p2.clone());
                Rc::new(move || {
                    let s = ctx.state.borrow();
                    let problem = if !s.encrypt {
                        None
                    } else if s.passphrase.chars().count() < 8 {
                        Some(tr("The passphrase needs at least 8 characters."))
                    } else if s.passphrase != s.passphrase2 {
                        Some(tr("The passphrases do not match."))
                    } else {
                        None
                    };
                    set_error(&p2, s.encrypt && !s.passphrase2.is_empty() && s.passphrase != s.passphrase2);
                    next.set_sensitive(problem.is_none());
                    hint.set_label(problem.unwrap_or(""));
                    hint.set_visible(problem.is_some());
                })
            };
            {
                let (ctx, v) = (ctx.clone(), validate.clone());
                exp.connect_enable_expansion_notify(move |e| {
                    ctx.state.borrow_mut().encrypt = e.enables_expansion();
                    v();
                });
            }
            {
                let (ctx, v) = (ctx.clone(), validate.clone());
                p1.connect_changed(move |e| {
                    ctx.state.borrow_mut().passphrase = e.text().into();
                    v();
                });
            }
            {
                let (ctx, v) = (ctx.clone(), validate.clone());
                p2.connect_changed(move |e| {
                    ctx.state.borrow_mut().passphrase2 = e.text().into();
                    v();
                });
            }
            validate();
        }
    }
    {
        let ctx = ctx.clone();
        next.connect_clicked(move |_| {
            let p = preparing_page(&ctx);
            ctx.nav.push(&p);
        });
    }
    setup_page("encryption", tr("Encryption"), &content, &next_bar(&next, &hint))
}

// ─────────────────────────── Preparing ───────────────────────────

fn build_config(ctx: &C) -> OobeConfig {
    let s = ctx.state.borrow();
    let w = ctx.wallpapers.get(s.wallpaper);
    OobeConfig {
        user: s.user.clone(),
        root_password: s.set_root_password.then(|| s.root_password.clone()),
        hostname: s.hostname.clone(),
        appearance: Appearance {
            color_scheme: if s.dark { "prefer-dark".into() } else { "default".into() },
            accent: s.accent.clone(),
            wallpaper: w.map(|w| w.light.clone()).unwrap_or_default(),
            wallpaper_dark: w.map(|w| w.dark.clone()).unwrap_or_default(),
        },
        location: s.location,
        automatic_timezone: s.automatic_timezone,
        language: s.language.clone(),
        country: s.country.clone(),
        encrypt_passphrase: (s.encrypt && ctx.encryption == Encryption::Available).then(|| s.passphrase.clone()),
    }
}

fn preparing_page(ctx: &C) -> adw::NavigationPage {
    let content = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(14).valign(gtk::Align::Center).vexpand(true).build();
    let visual = gtk::Stack::builder().transition_type(gtk::StackTransitionType::Crossfade).transition_duration(500).build();
    let spinner = adw::Spinner::builder().halign(gtk::Align::Center).width_request(64).height_request(64).build();
    spinner.add_css_class("prepare-spinner");
    let done = gtk::Image::from_icon_name("object-select-symbolic");
    done.add_css_class("done-icon");
    let failed = gtk::Image::from_icon_name("dialog-error-symbolic");
    failed.add_css_class("oobe-symbol");
    visual.add_named(&spinner, Some("busy"));
    visual.add_named(&done, Some("done"));
    visual.add_named(&failed, Some("failed"));
    content.append(&visual);
    let title = label(tr("Preparing MAN"), &["oobe-title"]);
    let detail = label(tr("Getting everything ready…"), &["dim-label"]);
    content.append(&title);
    content.append(&detail);

    let buttons = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(10).halign(gtk::Align::Center).margin_top(18).build();
    let reboot = pill(tr("_Reboot"), Some("suggested-action"));
    reboot.set_width_request(220);
    let back = pill(tr("_Go Back"), None);
    back.set_width_request(220);
    let show = pill(tr("_Show Details"), None);
    show.set_width_request(220);
    buttons.append(&reboot);
    buttons.append(&back);
    buttons.append(&show);
    let revealer = gtk::Revealer::builder().transition_type(gtk::RevealerTransitionType::SlideUp).transition_duration(450).child(&buttons).reveal_child(false).build();
    content.append(&revealer);

    let tv = adw::ToolbarView::new();
    tv.add_top_bar(&adw::HeaderBar::builder().show_end_title_buttons(false).show_start_title_buttons(false).show_title(false).show_back_button(false).build());
    let clamp = adw::Clamp::builder().maximum_size(520).child(&content).build();
    tv.set_content(Some(&clamp));
    let page = adw::NavigationPage::builder().title(tr("Preparing")).tag("preparing").child(&tv).can_pop(false).build();

    reboot.connect_clicked(|_| spawn_detached(&["systemctl", "reboot"]));
    {
        let ctx = ctx.clone();
        back.connect_clicked(move |_| {
            ctx.nav.pop_to_tag("account");
        });
    }
    let details = Rc::new(RefCell::new(String::new()));
    {
        let ctx = ctx.clone();
        let details = details.clone();
        show.connect_clicked(move |_| details_dialog(&ctx.window, tr("Setup Log"), &details.borrow()));
    }

    let ctx = ctx.clone();
    let config = build_config(&ctx);
    page.connect_shown(move |page| {
        if page.can_pop() {
            return;
        }
        let (ctx, config, visual, title, detail, revealer, reboot, back, show, details, page) =
            (ctx.clone(), config.clone(), visual.clone(), title.clone(), detail.clone(), revealer.clone(), reboot.clone(), back.clone(), show.clone(), details.clone(), page.clone());
        glib::spawn_future_local(async move {
            let started = std::time::Instant::now();
            let mut log = String::new();
            let res = backend::call(&Request::OobeApply { config }, |ev| match ev {
                Event::Step { title, .. } => detail.set_label(&format!("{}…", tr_dyn(title))),
                Event::Log { line } => {
                    log.push_str(line);
                    log.push('\n');
                }
                _ => {}
            })
            .await;
            // Let the spinner be seen for a moment even when setup is fast.
            let elapsed = started.elapsed();
            if elapsed < std::time::Duration::from_millis(2200) {
                glib::timeout_future(std::time::Duration::from_millis(2200) - elapsed).await;
            }
            match res {
                Ok(message) => {
                    ctx.state.borrow_mut().finished = true;
                    visual.set_visible_child_name("done");
                    title.set_label(tr("All Done!"));
                    detail.set_label(&message);
                    back.set_visible(false);
                    show.set_visible(false);
                    revealer.set_reveal_child(true);
                    reboot.grab_focus();
                }
                Err(f) => {
                    *details.borrow_mut() = format!("{}\n\n── {} ──\n{}", f.detail, tr("Log"), log);
                    visual.set_visible_child_name("failed");
                    title.set_label(tr("Something Went Wrong"));
                    detail.set_label(&f.message);
                    reboot.set_visible(false);
                    page.set_can_pop(true);
                    revealer.set_reveal_child(true);
                }
            }
        });
    });
    page
}

// ─────────────────────────── Window ───────────────────────────

pub fn run(app: &adw::Application) {
    // Start in the language the system was installed in.
    crate::i18n::set_language(crate::langui::system_language().locale);
    load_css();
    crate::langui::load_css();
    let provider = gtk::CssProvider::new();
    provider.load_from_string(CSS);
    let display = gdk::Display::default().expect("display");
    #[allow(deprecated)]
    gtk::style_context_add_provider_for_display(&display, &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    let accent_css = gtk::CssProvider::new();
    #[allow(deprecated)]
    gtk::style_context_add_provider_for_display(&display, &accent_css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1);

    let window = adw::ApplicationWindow::builder().application(app).title(tr("MAN Setup")).default_width(1280).default_height(800).deletable(false).build();
    let root = gtk::Overlay::new();
    root.add_css_class("oobe-backdrop");
    let backdrop = gtk::Stack::builder().transition_type(gtk::StackTransitionType::Crossfade).hexpand(true).vexpand(true).build();
    root.set_child(Some(&backdrop));
    let dim = gtk::Box::builder().hexpand(true).vexpand(true).can_target(false).build();
    dim.add_css_class("oobe-dim");
    root.add_overlay(&dim);

    let card = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .width_request(760)
        .height_request(640)
        .margin_top(24)
        .margin_bottom(24)
        .build();
    card.add_css_class("oobe-card");
    card.set_overflow(gtk::Overflow::Hidden);
    let nav = adw::NavigationView::new();
    nav.set_vexpand(true);
    let toasts = adw::ToastOverlay::new();
    toasts.set_child(Some(&nav));
    card.append(&toasts);
    let dots_box = gtk::Box::builder().spacing(8).halign(gtk::Align::Center).margin_bottom(18).build();
    let dots: Vec<gtk::Box> = STEPS
        .iter()
        .map(|_| {
            let d = gtk::Box::builder().valign(gtk::Align::Center).build();
            d.add_css_class("step-dot");
            dots_box.append(&d);
            d
        })
        .collect();
    card.append(&dots_box);
    root.add_overlay(&card);
    let fade = gtk::Picture::builder().can_target(false).visible(false).content_fit(gtk::ContentFit::Fill).build();
    root.add_overlay(&fade);
    window.set_content(Some(&root));

    let wallpapers = read_wallpapers();
    let count = wallpapers.len();
    let ctx: C = Rc::new(Ctx {
        window: window.clone(),
        nav: nav.clone(),
        toasts,
        root,
        fade,
        backdrop,
        dots,
        accent_css,
        wallpapers,
        encryption: detect_encryption(),
        homes: orphan_homes(),
        state: RefCell::new(State {
            language: crate::langui::system_language().locale.into(),
            country: crate::langui::system_language().country.into(),
            hostname: "man-pc".into(),
            accent: "blue".into(),
            location: false,
            automatic_timezone: false,
            ..Default::default()
        }),
        backdrop_serial: Cell::new(0),
        thumbs: RefCell::new(vec![None; count]),
        thumb_pictures: RefCell::new(vec![None; count]),
    });
    ctx.preload_thumbnails();
    ctx.apply_scheme();
    ctx.apply_accent();
    ctx.refresh_backdrop(false);
    {
        let ctx = ctx.clone();
        nav.connect_visible_page_notify(move |_| ctx.update_dots());
    }
    {
        let ctx = ctx.clone();
        window.connect_close_request(move |_| if ctx.state.borrow().finished { glib::Propagation::Proceed } else { glib::Propagation::Stop });
    }
    nav.add(&language_page(&ctx));
    ctx.update_dots();
    window.fullscreen();
    window.present();
}

pub fn main() -> glib::ExitCode {
    crate::i18n::init();
    let app = adw::Application::builder().application_id("com.man.Oobe").build();
    app.connect_activate(|app| {
        if let Some(w) = app.active_window() {
            w.present();
        } else {
            run(app);
        }
    });
    app.run()
}

#[cfg(test)]
mod tests {
    use super::nm_fields;

    #[test]
    fn nmcli_fields() {
        assert_eq!(nm_fields("*:My\\:Net:80:WPA2"), vec!["*", "My:Net", "80", "WPA2"]);
        assert_eq!(nm_fields(" :Cafe::"), vec![" ", "Cafe", "", ""]);
    }
}
