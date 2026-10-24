//! "App Installer": installs an app from a downloaded file, with the steps of
//! a classic installer shown on the left (Introduction, License, Destination
//! Select, Installation Type, Installation, Summary) and the app's icon below
//! them. Handles:
//!
//! - Debian packages (.deb): `apt-get install` through the root helper
//!   (man-app-installer-helper), dependencies from the apt sources.
//! - Flatpak bundles (.flatpak) and references (.flatpakref): `flatpak install`
//!   for the user or for all users.
//! - AppImages: moved to ~/AppImages (or /opt/AppImages for all users) and
//!   added to the apps with a menu entry and icon taken from the AppImage.
use crate::backend;
use crate::i18n::{tr, tr_dyn, trf};
use crate::ui::*;
use adw::prelude::*;
use gtk::{gio, glib};
use man_common::Event;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;

const APP_ID: &str = "com.man.AppInstaller";
const HELPER: &str = "/usr/libexec/man-installer/man-app-installer-helper";
const ALL_USERS_APPIMAGE_DIR: &str = "/opt/AppImages";

const CSS: &str = r#"
.installer-sidebar { background-color: var(--sidebar-bg-color); color: var(--sidebar-fg-color); }
.step-dot { min-width: 10px; min-height: 10px; border-radius: 99px; background-color: alpha(currentColor, 0.2); }
.step-dot.done { background-color: alpha(var(--accent-bg-color), 0.45); }
.step-dot.current { background-color: var(--accent-bg-color); }
.step-label.current { font-weight: bold; }
.installer-text { padding: 12px; }
.success-icon { color: var(--success-color); }
.error-icon { color: var(--error-color); }
"#;

// ─────────────────────────── The package ───────────────────────────

#[derive(Clone, Copy, PartialEq, Debug, Default)]
enum Kind {
    #[default]
    Deb,
    Flatpak,
    FlatpakRef,
    AppImage,
}

#[derive(Clone, Default, Debug)]
struct Package {
    kind: Kind,
    path: PathBuf,
    /// Package name, Flatpak app ID, or (AppImage) desktop file name.
    id: String,
    name: String,
    version: String,
    publisher: String,
    summary: String,
    description: String,
    homepage: String,
    license: String,
    license_text: String,
    /// Space the installed app takes (0: unknown).
    size: u64,
    icon: Option<PathBuf>,
    /// The app's menu entry: AppImage contents (rewritten when installing).
    desktop: Option<String>,
    /// .deb: the menu entry the package installs, for "Open".
    desktop_id: Option<String>,
    /// Installed version if the app is already installed ("" if unknown).
    installed: Option<String>,
    /// .flatpakref: where the app is downloaded from.
    source: String,
    /// Why the app cannot be installed here.
    problem: Option<String>,
    temp: PathBuf,
}

fn output(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).env("LC_ALL", "C.UTF-8").stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

fn detect(path: &Path) -> Option<Kind> {
    match path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref() {
        Some("deb") => return Some(Kind::Deb),
        Some("flatpak") => return Some(Kind::Flatpak),
        Some("flatpakref") => return Some(Kind::FlatpakRef),
        Some("appimage") => return Some(Kind::AppImage),
        _ => {}
    }
    let mut head = [0u8; 16];
    std::fs::File::open(path).ok()?.read_exact(&mut head).ok()?;
    if head.starts_with(b"!<arch>\ndebian-b") {
        Some(Kind::Deb)
    } else if head.starts_with(b"\x7fELF") && &head[8..10] == b"AI" {
        Some(Kind::AppImage)
    } else {
        None
    }
}

fn read_package(path: PathBuf, temp: PathBuf) -> Package {
    let _ = std::fs::create_dir_all(&temp);
    let mut p = Package { path: path.clone(), temp, ..Default::default() };
    p.name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    match detect(&path) {
        Some(k) => {
            p.kind = k;
            match k {
                Kind::Deb => read_deb(&mut p),
                Kind::Flatpak => read_flatpak(&mut p),
                Kind::FlatpakRef => read_flatpakref(&mut p),
                Kind::AppImage => read_appimage(&mut p),
            }
        }
        None => p.problem = Some(tr("This file is not an app that can be installed here (.deb, .flatpak, .flatpakref or AppImage).").into()),
    }
    if p.size == 0 && p.kind == Kind::AppImage {
        p.size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    }
    p
}

fn arch_problem(arch: &str) -> String {
    trf("This app is made for {arch} computers and does not work on this one (x86-64).", &[("arch", &arch)])
}

// ── Desktop entries ──

fn desktop_keyfile(text: &str) -> Option<glib::KeyFile> {
    let kf = glib::KeyFile::new();
    kf.load_from_data(text, glib::KeyFileFlags::NONE).ok()?;
    Some(kf)
}

fn desktop_value(kf: &glib::KeyFile, key: &str) -> String {
    kf.locale_string("Desktop Entry", key, None).map(|s| s.to_string()).unwrap_or_default()
}

/// Name, summary and icon name from a desktop entry.
fn apply_desktop(p: &mut Package, text: &str) -> String {
    let Some(kf) = desktop_keyfile(text) else { return String::new() };
    let name = desktop_value(&kf, "Name");
    if !name.is_empty() {
        p.name = name;
    }
    if p.summary.is_empty() {
        p.summary = desktop_value(&kf, "Comment");
    }
    kf.string("Desktop Entry", "Icon").map(|s| s.to_string()).unwrap_or_default()
}

// ── AppStream (Flatpak bundles, AppImages) ──

fn unescape_xml(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

/// Contents of the first `<tag>` without xml:lang (no nested tags of the same name).
fn xml_element<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let mut from = 0;
    while let Some(i) = xml[from..].find(&format!("<{tag}")) {
        let start = from + i;
        let rest = &xml[start + tag.len() + 1..];
        let end_open = rest.find('>')?;
        let attrs = &rest[..end_open];
        from = start + 1;
        if !(attrs.is_empty() || attrs.starts_with(' ')) || attrs.contains("xml:lang") || attrs.ends_with('/') {
            continue;
        }
        let body = &rest[end_open + 1..];
        return body.find(&format!("</{tag}>")).map(|e| &body[..e]);
    }
    None
}

fn strip_tags(s: &str) -> String {
    let s = s.replace("</p>", "\n\n").replace("<li>", "• ").replace("</li>", "\n");
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    reflow(&unescape_xml(&out))
}

/// Join lines broken for 80 columns into paragraphs; blank lines separate
/// paragraphs, list items ("•", "*", "-") keep their own line.
fn reflow(text: &str) -> String {
    let mut paragraphs: Vec<String> = vec![];
    for para in text.split("\n\n").map(str::trim).filter(|p| !p.is_empty()) {
        let mut out = String::new();
        for line in para.lines().map(str::trim).filter(|l| !l.is_empty()) {
            if !out.is_empty() {
                out.push(if line.starts_with(['•', '*', '-']) { '\n' } else { ' ' });
            }
            out.push_str(line);
        }
        paragraphs.push(out);
    }
    paragraphs.join("\n\n")
}

fn apply_appstream(p: &mut Package, xml: &str) {
    let text = |tag: &str| xml_element(xml, tag).map(strip_tags).filter(|s| !s.is_empty());
    if let Some(n) = text("name") {
        p.name = n;
    }
    if let Some(s) = text("summary") {
        p.summary = s;
    }
    if let Some(d) = text("description") {
        p.description = d;
    }
    if let Some(l) = text("project_license") {
        p.license = l;
    }
    p.publisher = text("developer_name").or_else(|| xml_element(xml, "developer").and_then(|d| xml_element(d, "name")).map(strip_tags)).unwrap_or(p.publisher.clone());
    if let Some(i) = xml.find("<url type=\"homepage\">") {
        let rest = &xml[i + 21..];
        p.homepage = rest.find("</url>").map(|e| unescape_xml(rest[..e].trim())).unwrap_or_default();
    }
    if p.version.is_empty() {
        if let Some(i) = xml.find("<release ") {
            let attrs = &xml[i..xml[i..].find('>').map_or(i, |e| i + e)];
            if let Some(v) = attrs.split("version=\"").nth(1).and_then(|v| v.split('"').next()) {
                p.version = v.to_string();
            }
        }
    }
}

fn gunzip(data: &[u8]) -> Option<String> {
    let conv = gio::ZlibDecompressor::new(gio::ZlibCompressorFormat::Gzip);
    let input = gio::MemoryInputStream::from_bytes(&glib::Bytes::from(data));
    let stream = gio::ConverterInputStream::new(&input, &conv);
    let mut out = vec![];
    loop {
        let chunk = stream.read_bytes(65536, gio::Cancellable::NONE).ok()?;
        if chunk.is_empty() {
            break;
        }
        out.extend_from_slice(&chunk);
    }
    String::from_utf8(out).ok()
}

// ── .deb ──

fn control_fields(text: &str) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = HashMap::new();
    let mut last = String::new();
    for line in text.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some(v) = map.get_mut(&last) {
                v.push('\n');
                v.push_str(line);
            }
        } else if let Some((k, v)) = line.split_once(':') {
            last = k.trim().to_string();
            map.insert(last.clone(), v.trim().to_string());
        }
    }
    map
}

fn deb_member(deb: &Path, member: &str) -> Option<Vec<u8>> {
    let mut dpkg = Command::new("dpkg-deb").arg("--fsys-tarfile").arg(deb).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    let out = Command::new("tar").args(["-xOf", "-", member]).stdin(dpkg.stdout.take()?).stderr(Stdio::null()).output().ok();
    let _ = dpkg.wait();
    out.filter(|o| o.status.success() && !o.stdout.is_empty()).map(|o| o.stdout)
}

/// The best icon file for `name` among `paths` (scalable first, then the largest).
fn best_icon<'a>(paths: impl Iterator<Item = &'a String>, name: &str) -> Option<String> {
    let mut best: Option<(u32, String)> = None;
    for p in paths {
        let file = Path::new(p);
        let (Some(stem), Some(ext)) = (file.file_stem().and_then(|s| s.to_str()), file.extension().and_then(|e| e.to_str())) else { continue };
        if stem != name || !["png", "svg"].contains(&ext) || !(p.contains("/icons/") || p.contains("/pixmaps/")) {
            continue;
        }
        let score = if ext == "svg" {
            10_000
        } else {
            p.split('/').find_map(|c| c.split_once('x').and_then(|(w, _)| w.parse::<u32>().ok())).unwrap_or(48)
        };
        if best.as_ref().is_none_or(|(s, _)| score > *s) {
            best = Some((score, p.clone()));
        }
    }
    best.map(|(_, p)| p)
}

fn save_icon(p: &mut Package, data: &[u8], ext: &str) {
    let dest = p.temp.join(format!("icon.{ext}"));
    if std::fs::write(&dest, data).is_ok() {
        p.icon = Some(dest);
    }
}

fn read_deb(p: &mut Package) {
    let path = p.path.to_string_lossy().into_owned();
    let Some(control) = output("dpkg-deb", &["-f", &path]) else {
        p.problem = Some(tr("The file could not be read. It may be damaged.").into());
        return;
    };
    let f = control_fields(&control);
    let get = |k: &str| f.get(k).cloned().unwrap_or_default();
    p.id = get("Package");
    p.name = p.id.clone();
    p.version = get("Version");
    p.publisher = get("Maintainer").split('<').next().unwrap_or("").trim().to_string();
    p.homepage = get("Homepage");
    p.size = get("Installed-Size").trim().parse::<u64>().unwrap_or(0) * 1024;
    let desc = get("Description");
    let mut lines = desc.lines();
    p.summary = lines.next().unwrap_or("").trim().to_string();
    p.description = reflow(&lines.map(|l| if l.trim() == "." { "" } else { l.strip_prefix(' ').unwrap_or(l) }).collect::<Vec<_>>().join("\n"));
    let arch = get("Architecture");
    if arch != "amd64" && arch != "all" {
        p.problem = Some(arch_problem(&arch));
    }

    let listing = output("dpkg-deb", &["-c", &path]).unwrap_or_default();
    let paths: Vec<String> = listing
        .lines()
        .filter_map(|l| l.split_whitespace().skip(5).collect::<Vec<_>>().join(" ").split(" -> ").next().map(|s| s.trim_start_matches('.').to_string()))
        .collect();
    let mut icon_name = String::new();
    // The menu entry named like the package first (xterm.desktop before uxterm.desktop).
    let mut desktops: Vec<&String> = paths.iter().filter(|p| p.starts_with("/usr/share/applications/") && p.ends_with(".desktop")).collect();
    let (own, debian) = (format!("/{}.desktop", p.id), format!("-{}.desktop", p.id));
    desktops.sort_by_key(|d| !(d.ends_with(&own) || d.ends_with(&debian)));
    for d in desktops {
        let Some(text) = deb_member(&p.path, &format!(".{d}")).and_then(|b| String::from_utf8(b).ok()) else { continue };
        if desktop_keyfile(&text).is_some_and(|kf| kf.boolean("Desktop Entry", "NoDisplay").unwrap_or(false)) {
            continue;
        }
        icon_name = apply_desktop(p, &text);
        p.desktop_id = Path::new(d).file_name().map(|n| n.to_string_lossy().into_owned());
        break;
    }
    if !icon_name.is_empty() {
        let found = if icon_name.starts_with('/') { paths.iter().find(|x| **x == icon_name).cloned() } else { best_icon(paths.iter(), &icon_name) };
        if let Some(icon) = found {
            if let Some(data) = deb_member(&p.path, &format!(".{icon}")) {
                let ext = if icon.ends_with(".svg") { "svg" } else { "png" };
                save_icon(p, &data, ext);
            }
        }
    }
    let copyright = format!("/usr/share/doc/{}/copyright", p.id);
    if paths.contains(&copyright) {
        p.license_text = deb_member(&p.path, &format!(".{copyright}")).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
    }
    if let Some(status) = output("dpkg-query", &["-W", "-f=${Status}|${Version}", &p.id]) {
        if let Some((s, v)) = status.split_once('|') {
            if s.ends_with("installed") && !s.contains("not-installed") && !s.contains("config-files") {
                p.installed = Some(v.to_string());
            }
        }
    }
}

// ── Flatpak ──

/// The metadata dictionary at the start of a Flatpak bundle (an OSTree
/// static delta superblock, a GVariant tuple whose first member is a{sv}).
fn bundle_metadata(path: &Path) -> Option<glib::VariantDict> {
    let mut f = std::fs::File::open(path).ok()?;
    let n = f.metadata().ok()?.len();
    // Framing offsets of a GVariant container sit at its end; the offset of
    // the first variable-size member comes last.
    let w: u64 = if n <= 0xff { 1 } else if n <= 0xffff { 2 } else if n <= 0xffff_ffff { 4 } else { 8 };
    f.seek(SeekFrom::Start(n - w)).ok()?;
    let mut buf = [0u8; 8];
    f.read_exact(&mut buf[..w as usize]).ok()?;
    let end = u64::from_le_bytes(buf);
    if end == 0 || end > n || end > 256 << 20 {
        return None;
    }
    let mut meta = vec![0u8; end as usize];
    f.seek(SeekFrom::Start(0)).ok()?;
    f.read_exact(&mut meta).ok()?;
    let v = glib::Variant::from_bytes_with_type(&glib::Bytes::from_owned(meta), glib::VariantTy::new("a{sv}").ok()?);
    v.is_normal_form().then(|| glib::VariantDict::new(Some(&v)))
}

fn apply_ref(p: &mut Package, flatpak_ref: &str) {
    let parts: Vec<&str> = flatpak_ref.split('/').collect();
    if parts.len() == 4 {
        p.id = parts[1].to_string();
        p.name = p.id.rsplit('.').next().unwrap_or(&p.id).to_string();
        if parts[2] != "x86_64" {
            p.problem = Some(arch_problem(parts[2]));
        }
    }
}

fn flatpak_installed(p: &mut Package) {
    if !p.id.is_empty() && output("flatpak", &["info", &p.id]).is_some() {
        p.installed = Some(String::new());
    }
}

fn read_flatpak(p: &mut Package) {
    let Some(meta) = bundle_metadata(&p.path) else {
        p.problem = Some(tr("The file could not be read. It may be damaged.").into());
        return;
    };
    let bytes = |key: &str| meta.lookup_value(key, None).map(|v| v.data_as_bytes().to_vec());
    if let Some(r) = meta.lookup_value("ref", None).and_then(|v| v.str().map(String::from)) {
        apply_ref(p, &r);
    }
    if let Some(size) = meta.lookup_value("installed-size", None).and_then(|v| v.get::<u64>()) {
        p.size = u64::from_be(size);
    }
    if let Some(xml) = bytes("appdata").and_then(|d| gunzip(&d)) {
        apply_appstream(p, &xml);
    }
    if let Some(icon) = bytes("icon-128").or_else(|| bytes("icon-64")).filter(|d| !d.is_empty()) {
        save_icon(p, &icon, "png");
    }
    flatpak_installed(p);
}

fn read_flatpakref(p: &mut Package) {
    let kf = glib::KeyFile::new();
    if kf.load_from_file(&p.path, glib::KeyFileFlags::NONE).is_err() {
        p.problem = Some(tr("The file could not be read. It may be damaged.").into());
        return;
    }
    let get = |k: &str| kf.locale_string("Flatpak Ref", k, None).map(|s| s.to_string()).unwrap_or_default();
    p.id = get("Name");
    p.name = Some(get("Title")).filter(|t| !t.is_empty()).unwrap_or_else(|| p.id.clone());
    p.summary = get("Comment");
    p.description = get("Description");
    p.homepage = get("Homepage");
    p.version = get("Branch");
    p.source = get("Url").split('/').nth(2).unwrap_or("").to_string();
    if p.id.is_empty() {
        p.problem = Some(tr("The file could not be read. It may be damaged.").into());
    }
    flatpak_installed(p);
}

// ── AppImage ──

/// Where the squashfs image of a type 2 AppImage starts: right after its ELF runtime.
fn squashfs_offset(head: &[u8; 64]) -> Option<u64> {
    let u16le = |o: usize| u16::from_le_bytes([head[o], head[o + 1]]) as u64;
    let u32le = |o: usize| u32::from_le_bytes(head[o..o + 4].try_into().unwrap()) as u64;
    let u64le = |o: usize| u64::from_le_bytes(head[o..o + 8].try_into().unwrap());
    match head[4] {
        2 => Some(u64le(0x28) + u16le(0x3a) * u16le(0x3c)),
        1 => Some(u32le(0x20) + u16le(0x2e) * u16le(0x30)),
        _ => None,
    }
}

fn unsquash(p: &Package, offset: u64, dir: &Path, patterns: &[&str]) {
    let _ = Command::new("unsquashfs")
        .args(["-q", "-n", "-f", "-no-xattrs", "-o", &offset.to_string(), "-d"])
        .arg(dir)
        .arg(&p.path)
        .args(patterns)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn read_appimage(p: &mut Package) {
    let mut head = [0u8; 64];
    if std::fs::File::open(&p.path).and_then(|mut f| f.read_exact(&mut head)).is_err() || !head.starts_with(b"\x7fELF") {
        p.problem = Some(tr("This is not an AppImage.").into());
        return;
    }
    let machine = u16::from_le_bytes([head[0x12], head[0x13]]);
    if machine != 0x3e {
        let arch = match machine { 0xb7 => "ARM64", 0x28 => "ARM", 0x03 => "x86 (32-bit)", _ => "?" };
        p.problem = Some(arch_problem(arch));
    }
    // Version from the file name ("App-1.2.3-x86_64.AppImage") until the app says better.
    p.version = p.name.split(['-', '_']).find(|s| s.chars().next().is_some_and(|c| c.is_ascii_digit()) && s.contains('.')).unwrap_or("").to_string();
    p.name = p.name.split(['-', '_']).next().unwrap_or(&p.name).to_string();
    p.id = p.name.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '.').collect();
    let Some(offset) = (&head[8..11] == b"AI\x02" || &head[8..10] != b"AI").then(|| squashfs_offset(&head)).flatten() else { return };

    let dir = p.temp.join("appimage");
    unsquash(p, offset, &dir, &["*.desktop", ".DirIcon", "*.png", "*.svg", "usr/share/metainfo/*.xml", "usr/share/appdata/*.xml"]);
    let entries: Vec<PathBuf> = std::fs::read_dir(&dir).map(|d| d.flatten().map(|e| e.path()).collect()).unwrap_or_default();
    let mut icon_name = String::new();
    if let Some(desktop) = entries.iter().find(|e| e.extension().is_some_and(|x| x == "desktop")) {
        if let Ok(text) = std::fs::read_to_string(desktop) {
            icon_name = apply_desktop(p, &text);
            if let Some(kf) = desktop_keyfile(&text) {
                if let Ok(v) = kf.string("Desktop Entry", "X-AppImage-Version") {
                    p.version = v.to_string();
                }
            }
            p.id = desktop.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or(p.id.clone());
            p.desktop = Some(text);
        }
    }
    for sub in ["usr/share/metainfo", "usr/share/appdata"] {
        if let Some(xml) = std::fs::read_dir(dir.join(sub)).ok().and_then(|d| d.flatten().find_map(|e| std::fs::read_to_string(e.path()).ok())) {
            apply_appstream(p, &xml);
            break;
        }
    }
    // The icon: the one the menu entry names, else .DirIcon (often a link to it).
    let mut icon = ["svg", "png"].iter().map(|ext| dir.join(format!("{icon_name}.{ext}"))).find(|f| !icon_name.is_empty() && f.is_file());
    if icon.is_none() {
        let dir_icon = dir.join(".DirIcon");
        if let Ok(target) = std::fs::read_link(&dir_icon) {
            let rel = target.to_string_lossy().trim_start_matches('/').to_string();
            unsquash(p, offset, &dir, &[&rel]);
            icon = Some(dir.join(rel)).filter(|f| f.is_file());
        } else if dir_icon.is_file() {
            icon = Some(dir_icon);
        }
    }
    if let Some(i) = icon {
        if let Ok(data) = std::fs::read(&i) {
            let ext = if data.starts_with(b"\x89PNG") { "png" } else { "svg" };
            save_icon(p, &data, ext);
        }
    }
    let id = p.id.clone();
    if !id.is_empty() && appimage_desktop_path(false, &id).exists() {
        p.installed = Some(String::new());
    }
}

fn appimage_dir(all_users: bool) -> PathBuf {
    if all_users { PathBuf::from(ALL_USERS_APPIMAGE_DIR) } else { glib::home_dir().join("AppImages") }
}

fn appimage_desktop_path(all_users: bool, id: &str) -> PathBuf {
    let dir = if all_users { PathBuf::from("/usr/local/share/applications") } else { glib::user_data_dir().join("applications") };
    dir.join(format!("appimage-{id}.desktop"))
}

fn icon_dest(p: &Package, all_users: bool) -> Option<PathBuf> {
    let ext = p.icon.as_ref()?.extension()?.to_string_lossy().into_owned();
    Some(appimage_dir(all_users).join(".icons").join(format!("{}.{ext}", p.id)))
}

fn quote_exec(path: &Path) -> String {
    let s = path.to_string_lossy();
    if s.chars().all(|c| c.is_ascii_alphanumeric() || "/._-+".contains(c)) {
        s.into_owned()
    } else {
        format!("\"{}\"", s.replace('\\', "\\\\\\\\").replace('"', "\\\"").replace('`', "\\`").replace('$', "\\$"))
    }
}

/// The menu entry of an installed AppImage: the app's own entry with Exec
/// and Icon pointing to the installed files.
fn appimage_desktop(p: &Package, all_users: bool) -> String {
    let file = appimage_dir(all_users).join(p.path.file_name().unwrap_or_default());
    let exec = quote_exec(&file);
    let icon = icon_dest(p, all_users).map(|i| i.to_string_lossy().into_owned());
    let mut out = vec![];
    let mut group = String::new();
    match &p.desktop {
        Some(text) => {
            for line in text.lines() {
                if line.starts_with('[') {
                    group = line.to_string();
                    out.push(line.to_string());
                    if group == "[Desktop Entry]" {
                        out.push(format!("X-MAN-AppImage={}", file.display()));
                    }
                    continue;
                }
                if line.starts_with("TryExec=") || line.starts_with("X-MAN-AppImage=") {
                    continue;
                }
                if let Some(cmd) = line.strip_prefix("Exec=") {
                    // Replace the program, keep its arguments (%U, --flags…).
                    let rest = if let Some(q) = cmd.strip_prefix('"') { q.split_once('"').map(|(_, r)| r).unwrap_or("") } else { cmd.split_once(' ').map(|(_, r)| r).unwrap_or("") };
                    out.push(format!("Exec={exec}{}{}", if rest.trim().is_empty() { "" } else { " " }, rest.trim()));
                } else if line.starts_with("Icon=") && group == "[Desktop Entry]" {
                    out.push(format!("Icon={}", icon.clone().unwrap_or_else(|| line[5..].to_string())));
                } else {
                    out.push(line.to_string());
                }
            }
        }
        None => {
            out.push("[Desktop Entry]".into());
            out.push(format!("X-MAN-AppImage={}", file.display()));
            out.push("Type=Application".into());
            out.push(format!("Name={}", p.name));
            out.push(format!("Exec={exec} %U"));
            out.push(format!("Icon={}", icon.unwrap_or_else(|| "application-x-executable".into())));
            out.push("Terminal=false".into());
            out.push("Categories=Utility;".into());
        }
    }
    out.join("\n") + "\n"
}

fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    if from == to || std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    let tmp = to.with_extension("part");
    std::fs::copy(from, &tmp)?;
    std::fs::rename(&tmp, to)?;
    std::fs::remove_file(from)
}

/// AppImage for the user: everything happens in the home folder.
fn install_appimage_user(p: &Package) -> Result<PathBuf, String> {
    use std::os::unix::fs::PermissionsExt;
    let dir = appimage_dir(false);
    let dest = dir.join(p.path.file_name().unwrap_or_default());
    let desktop = appimage_desktop_path(false, &p.id);
    (|| -> std::io::Result<()> {
        std::fs::create_dir_all(dir.join(".icons"))?;
        move_file(&p.path, &dest)?;
        std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))?;
        if let (Some(src), Some(to)) = (&p.icon, icon_dest(p, false)) {
            std::fs::copy(src, to)?;
        }
        std::fs::create_dir_all(desktop.parent().unwrap())?;
        std::fs::write(&desktop, appimage_desktop(p, false))?;
        let _ = Command::new("update-desktop-database").arg("-q").arg(desktop.parent().unwrap()).status();
        Ok(())
    })()
    .map_err(|e| e.to_string())?;
    Ok(desktop)
}

/// For deb installs: what else apt would install, or why it cannot.
fn simulate_deb(path: &Path, own: &str) -> Result<Vec<String>, String> {
    let out = Command::new("apt-get")
        .args(["install", "-s", "--no-install-recommends"])
        .arg(path)
        .env("LC_ALL", "C.UTF-8")
        .stdin(Stdio::null())
        .output()
        .map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    if !out.status.success() {
        let lines: Vec<&str> = text.lines().filter(|l| l.contains("Depends:") || l.contains("Conflicts:") || l.starts_with("E:")).collect();
        return Err(if lines.is_empty() { text.trim().to_string() } else { lines.join("\n") });
    }
    Ok(text.lines().filter_map(|l| l.strip_prefix("Inst ")).filter_map(|l| l.split_whitespace().next()).filter(|n| *n != own).map(String::from).collect())
}

// ─────────────────────────── The window ───────────────────────────

#[derive(Clone, Copy, PartialEq, Debug)]
enum Step {
    Introduction,
    License,
    Destination,
    Type,
    Installation,
    Summary,
}

impl Step {
    fn title(self) -> &'static str {
        match self {
            Step::Introduction => tr("Introduction"),
            Step::License => tr("License"),
            Step::Destination => tr("Destination Select"),
            Step::Type => tr("Installation Type"),
            Step::Installation => tr("Installation"),
            Step::Summary => tr("Summary"),
        }
    }
}

struct Ctx {
    window: adw::ApplicationWindow,
    title: adw::WindowTitle,
    steps: RefCell<Vec<Step>>,
    current: Cell<usize>,
    step_list: gtk::Box,
    side_icon: gtk::Box,
    heading: gtk::Label,
    content: adw::Bin,
    back: gtk::Button,
    next: gtk::Button,
    extra: gtk::Button,
    on_next: RefCell<Option<Rc<dyn Fn()>>>,
    on_extra: RefCell<Option<Rc<dyn Fn()>>>,
    pkg: RefCell<Package>,
    all_users: Cell<bool>,
    busy: Cell<bool>,
    deb_ok: Cell<bool>,
    installed_desktop: RefCell<Option<PathBuf>>,
}

type C = Rc<Ctx>;

impl Ctx {
    fn pkg(&self) -> Package {
        self.pkg.borrow().clone()
    }
}

fn wrap_label(text: &str, classes: &[&str]) -> gtk::Label {
    let l = gtk::Label::builder().label(text).wrap(true).xalign(0.0).selectable(false).build();
    l.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    for c in classes {
        l.add_css_class(c);
    }
    l
}

fn property_row(title: &str, value: &str) -> adw::ActionRow {
    let row = adw::ActionRow::builder().title(title).subtitle(glib::markup_escape_text(value)).subtitle_selectable(true).build();
    row.add_css_class("property");
    row
}

fn scrolled(child: &impl IsA<gtk::Widget>) -> gtk::ScrolledWindow {
    gtk::ScrolledWindow::builder().hscrollbar_policy(gtk::PolicyType::Never).vexpand(true).child(child).build()
}

fn render_steps(ctx: &C) {
    while let Some(c) = ctx.step_list.first_child() {
        ctx.step_list.remove(&c);
    }
    let current = ctx.current.get();
    for (i, s) in ctx.steps.borrow().iter().enumerate() {
        let row = gtk::Box::builder().spacing(12).build();
        let dot = gtk::Box::builder().valign(gtk::Align::Center).build();
        dot.add_css_class("step-dot");
        let lbl = gtk::Label::builder().label(s.title()).xalign(0.0).build();
        lbl.add_css_class("step-label");
        if i == current {
            dot.add_css_class("current");
            lbl.add_css_class("current");
        } else if i < current {
            dot.add_css_class("done");
        } else {
            lbl.add_css_class("dim-label");
        }
        row.append(&dot);
        row.append(&lbl);
        ctx.step_list.append(&row);
    }
}

fn set_side_icon(ctx: &C) {
    while let Some(c) = ctx.side_icon.first_child() {
        ctx.side_icon.remove(&c);
    }
    let p = ctx.pkg();
    let w: gtk::Widget = match p.icon.as_ref() {
        Some(file) => gtk::Picture::builder().file(&gio::File::for_path(file)).content_fit(gtk::ContentFit::Contain).width_request(128).height_request(128).can_shrink(true).build().upcast(),
        None => {
            let i = gtk::Image::from_icon_name(APP_ID);
            i.set_pixel_size(128);
            i.upcast()
        }
    };
    ctx.side_icon.append(&w);
}

/// Show step `index`: heading, page content and what the buttons do.
fn show(ctx: &C, index: usize) {
    ctx.current.set(index);
    render_steps(ctx);
    let step = ctx.steps.borrow()[index];
    ctx.extra.set_visible(false);
    *ctx.on_extra.borrow_mut() = None;
    ctx.back.set_visible(true);
    ctx.back.set_sensitive(index > 0);
    ctx.next.set_sensitive(true);
    ctx.next.set_label(tr("_Continue"));
    let p = ctx.pkg();
    let next_step = {
        let ctx = ctx.clone();
        Rc::new(move || show(&ctx, index + 1)) as Rc<dyn Fn()>
    };
    *ctx.on_next.borrow_mut() = Some(next_step.clone());
    let content: gtk::Widget = match step {
        Step::Introduction => {
            ctx.heading.set_label(&trf("Welcome to the {name} Installer", &[("name", &p.name)]));
            intro_page(ctx, &p).upcast()
        }
        Step::License => {
            ctx.heading.set_label(tr("Software License Agreement"));
            let view = gtk::TextView::builder().editable(false).cursor_visible(false).wrap_mode(gtk::WrapMode::WordChar).build();
            view.add_css_class("installer-text");
            view.buffer().set_text(&p.license_text);
            let s = scrolled(&view);
            s.add_css_class("card");
            s.set_overflow(gtk::Overflow::Hidden);
            let ctx2 = ctx.clone();
            *ctx.on_next.borrow_mut() = Some(Rc::new(move || {
                let dialog = adw::AlertDialog::new(
                    Some(tr("Do You Agree to the License?")),
                    Some(tr("To continue installing, you must agree to the terms of the software license agreement.")),
                );
                dialog.add_response("read", tr("_Read License"));
                dialog.add_response("agree", tr("_Agree"));
                dialog.set_response_appearance("agree", adw::ResponseAppearance::Suggested);
                dialog.set_close_response("read");
                let (ctx, next) = (ctx2.clone(), next_step.clone());
                glib::spawn_future_local(async move {
                    if dialog.choose_future(Some(&ctx.window)).await == "agree" {
                        next();
                    }
                });
            }));
            s.upcast()
        }
        Step::Destination => {
            ctx.heading.set_label(tr("Select a Destination"));
            destination_page(ctx, &p).upcast()
        }
        Step::Type => {
            ctx.heading.set_label(tr("Standard Install"));
            ctx.next.set_label(tr("_Install"));
            let ctx2 = ctx.clone();
            *ctx.on_next.borrow_mut() = Some(Rc::new(move || install(&ctx2)));
            type_page(ctx, &p).upcast()
        }
        Step::Installation | Step::Summary => unreachable!("built by install()"),
    };
    ctx.content.set_child(Some(&content));
    ctx.next.grab_focus();
}

fn kind_name(p: &Package) -> String {
    match p.kind {
        Kind::Deb => tr("Debian package (.deb)").into(),
        Kind::Flatpak => tr("Flatpak app").into(),
        Kind::FlatpakRef => trf("Flatpak app from {source}", &[("source", &p.source)]),
        Kind::AppImage => tr("AppImage").into(),
    }
}

fn notice(icon_name: &str, text: &str, class: &str) -> gtk::Box {
    let b = gtk::Box::builder().spacing(12).build();
    b.add_css_class("card");
    b.add_css_class("summary-warning");
    let i = gtk::Image::from_icon_name(icon_name);
    i.add_css_class(class);
    i.set_valign(gtk::Align::Start);
    b.append(&i);
    b.append(&wrap_label(text, &[]));
    b
}

fn intro_page(ctx: &C, p: &Package) -> gtk::ScrolledWindow {
    let b = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(18).margin_end(6).build();
    b.append(&wrap_label(tr("You will be guided through the steps necessary to install this software."), &[]));
    if !p.summary.is_empty() {
        b.append(&wrap_label(&p.summary, &["heading"]));
    }
    if !p.description.is_empty() {
        b.append(&wrap_label(&p.description, &[]));
    }
    let list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).build();
    list.add_css_class("boxed-list");
    let mut rows: Vec<(&str, String)> = vec![];
    if !p.version.is_empty() {
        rows.push((tr("Version"), p.version.clone()));
    }
    if !p.publisher.is_empty() {
        rows.push((tr("Publisher"), p.publisher.clone()));
    }
    rows.push((tr("Type"), kind_name(p)));
    if p.size > 0 {
        rows.push((tr("Size"), glib::format_size(p.size).to_string()));
    }
    if !p.license.is_empty() {
        rows.push((tr("License"), p.license.clone()));
    }
    if !p.homepage.is_empty() {
        rows.push((tr("Website"), p.homepage.clone()));
    }
    rows.push((tr("File"), p.path.display().to_string()));
    for (t, v) in rows {
        list.append(&property_row(t, &v));
    }
    b.append(&list);
    if let Some(problem) = &p.problem {
        b.append(&notice("dialog-error-symbolic", problem, "error-icon"));
        ctx.next.set_sensitive(false);
    } else {
        if p.installed.is_some() {
            b.append(&notice("dialog-information-symbolic", tr("This app is installed already. Installing it again replaces it."), "dim-label"));
        }
        if p.kind == Kind::Deb {
            b.append(&notice("dialog-warning-symbolic", tr("Apps from .deb packages become part of the system. When MAN is updated they are removed and need to be installed again. Flatpak apps and AppImages are kept."), "warning"));
        }
    }
    b.append(&wrap_label(tr("Only install apps from sources you trust."), &["caption", "dim-label"]));
    scrolled(&b)
}

fn destination_page(ctx: &C, p: &Package) -> gtk::Box {
    let b = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(18).build();
    b.append(&wrap_label(&trf("Where do you want to install {name}?", &[("name", &p.name)]), &[]));
    let list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).build();
    list.add_css_class("boxed-list");
    let (me_sub, all_sub) = match p.kind {
        Kind::AppImage => (
            format!("{}  ·  {}", appimage_dir(false).display(), tr("Only your account can use the app.")),
            format!("{}  ·  {}", ALL_USERS_APPIMAGE_DIR, tr("Needs administrator rights.")),
        ),
        _ => (tr("Only your account can use the app.").to_string(), tr("Needs administrator rights.").to_string()),
    };
    let me = gtk::CheckButton::new();
    let all = gtk::CheckButton::new();
    all.set_group(Some(&me));
    for (check, title, sub, active) in [(&me, tr("Only for Me"), me_sub, !ctx.all_users.get()), (&all, tr("For All Users of This Computer"), all_sub, ctx.all_users.get())] {
        let row = adw::ActionRow::builder().title(title).subtitle(glib::markup_escape_text(&sub)).activatable_widget(check).build();
        row.add_prefix(check);
        check.set_active(active);
        list.append(&row);
    }
    let c = ctx.clone();
    all.connect_toggled(move |a| c.all_users.set(a.is_active()));
    b.append(&list);
    b
}

fn type_page(ctx: &C, p: &Package) -> gtk::ScrolledWindow {
    let b = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(18).build();
    if p.size > 0 {
        b.append(&wrap_label(&trf("This will take {size} of space on your computer.", &[("size", &glib::format_size(p.size))]), &["title-4"]));
    } else if !p.source.is_empty() {
        b.append(&wrap_label(&trf("The app is downloaded from {source}.", &[("source", &p.source)]), &["title-4"]));
    }
    let who = if p.kind == Kind::Deb || ctx.all_users.get() { tr("The app is installed for all users of this computer.") } else { tr("The app is installed only for you.") };
    b.append(&wrap_label(who, &[]));
    if p.kind == Kind::AppImage {
        b.append(&wrap_label(&trf("The AppImage is moved to {folder} and added to your apps.", &[("folder", &appimage_dir(ctx.all_users.get()).display())]), &[]));
    }
    b.append(&wrap_label(&trf("Click Install to install {name}.", &[("name", &p.name)]), &["dim-label"]));
    if p.kind == Kind::Deb {
        // Ask apt what else is needed before asking for a password.
        let status = gtk::Box::builder().spacing(12).build();
        let spinner = adw::Spinner::new();
        status.append(&spinner);
        status.append(&wrap_label(tr("Checking what else the app needs…"), &["dim-label"]));
        b.append(&status);
        ctx.next.set_sensitive(false);
        let (ctx, b, path, id) = (ctx.clone(), b.clone(), p.path.clone(), p.id.clone());
        glib::spawn_future_local(async move {
            let res = gio::spawn_blocking(move || simulate_deb(&path, &id)).await.unwrap_or_else(|_| Err(String::new()));
            b.remove(&status);
            match res {
                Ok(extra) => {
                    if !extra.is_empty() {
                        b.append(&wrap_label(&trf("Also installs the packages it needs: {list}", &[("list", &extra.join(", "))]), &[]));
                    }
                    ctx.deb_ok.set(true);
                    ctx.next.set_sensitive(true);
                }
                Err(why) => {
                    b.append(&notice("dialog-warning-symbolic", tr("Some packages the app needs are not available yet. Installing gets the newest package lists first; if they are still missing, the app cannot be installed."), "warning"));
                    let details = wrap_label(&why, &["caption", "monospace"]);
                    details.set_selectable(true);
                    b.append(&details);
                    ctx.deb_ok.set(true);
                    ctx.next.set_sensitive(true);
                }
            }
        });
    }
    scrolled(&b)
}

// ─────────────────────────── Installing ───────────────────────────

async fn run_streaming(argv: &[String], mut on_line: impl FnMut(&str)) -> Result<(), String> {
    let os: Vec<&std::ffi::OsStr> = argv.iter().map(std::ffi::OsStr::new).collect();
    let proc = gio::Subprocess::newv(&os, gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_MERGE).map_err(|e| e.to_string())?;
    let out = proc.stdout_pipe().expect("stdout pipe");
    let mut pending = String::new();
    let mut tail: Vec<String> = vec![];
    loop {
        let chunk = out.read_bytes_future(4096, glib::Priority::DEFAULT).await.map_err(|e| e.to_string())?;
        if chunk.is_empty() {
            break;
        }
        pending.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(i) = pending.find(['\r', '\n']) {
            let line: String = pending.drain(..=i).collect();
            let line = line.trim();
            if !line.is_empty() {
                on_line(line);
                tail.push(line.to_string());
                if tail.len() > 12 {
                    tail.remove(0);
                }
            }
        }
    }
    let _ = proc.wait_future().await;
    if proc.has_exited() && proc.exit_status() == 0 {
        Ok(())
    } else {
        Err(tail.join("\n"))
    }
}

fn percent(line: &str) -> Option<f64> {
    let i = line.rfind('%')?;
    let digits: String = line[..i].chars().rev().take_while(|c| c.is_ascii_digit()).collect::<Vec<_>>().into_iter().rev().collect();
    digits.parse::<f64>().ok().filter(|v| *v <= 100.0).map(|v| v / 100.0)
}

fn install(ctx: &C) {
    if ctx.pkg().kind == Kind::Deb && !ctx.deb_ok.get() {
        return;
    }
    let index = ctx.steps.borrow().iter().position(|s| *s == Step::Installation).unwrap_or(0);
    ctx.current.set(index);
    render_steps(ctx);
    let p = ctx.pkg();
    ctx.heading.set_label(&trf("Installing {name}…", &[("name", &p.name)]));
    ctx.back.set_sensitive(false);
    ctx.next.set_sensitive(false);
    ctx.busy.set(true);

    let b = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(12).valign(gtk::Align::Center).build();
    let status = wrap_label(tr("Preparing…"), &[]);
    let bar = gtk::ProgressBar::new();
    bar.add_css_class("install-progress");
    let log_view = gtk::TextView::builder().editable(false).cursor_visible(false).wrap_mode(gtk::WrapMode::WordChar).build();
    log_view.add_css_class("log-view");
    log_view.add_css_class("card");
    let log_scroll = gtk::ScrolledWindow::builder().min_content_height(160).child(&log_view).build();
    let expander = gtk::Expander::builder().label(tr("Details")).child(&log_scroll).build();
    b.append(&status);
    b.append(&bar);
    b.append(&expander);
    ctx.content.set_child(Some(&b));

    let pulse = {
        let bar = bar.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
            if bar.fraction() == 0.0 || bar.fraction() >= 1.0 {
                bar.pulse();
            }
            glib::ControlFlow::Continue
        })
    };
    let buffer = log_view.buffer();
    let log = {
        let (buffer, view) = (buffer.clone(), log_view.clone());
        move |line: &str| {
            let mut end = buffer.end_iter();
            buffer.insert(&mut end, &format!("{line}\n"));
            let mark = buffer.create_mark(None, &buffer.end_iter(), false);
            view.scroll_mark_onscreen(&mark);
            buffer.delete_mark(&mark);
        }
    };
    let ctx = ctx.clone();
    let all = ctx.all_users.get();
    glib::spawn_future_local(async move {
        let on_event = |ev: &Event| match ev {
            Event::Progress { fraction, detail } => {
                bar.set_fraction(fraction.clamp(0.01, 1.0));
                status.set_label(&tr_dyn(detail));
            }
            Event::Step { title, .. } => status.set_label(&tr_dyn(title)),
            Event::Log { line } => log(line),
            _ => {}
        };
        let result: Result<(), (String, String)> = match p.kind {
            Kind::Deb => {
                status.set_label(tr("Installing the package…"));
                let req = serde_json::json!({ "op": "deb_install", "path": p.path });
                backend::call_helper(HELPER, req.to_string().into_bytes(), on_event).await.map(|_| ()).map_err(|f| (f.message, f.detail))
            }
            Kind::AppImage if all => {
                let req = serde_json::json!({
                    "op": "appimage_install",
                    "path": p.path,
                    "id": p.id,
                    "desktop": appimage_desktop(&p, true),
                    "icon": p.icon.clone().unwrap_or_default(),
                });
                let r = backend::call_helper(HELPER, req.to_string().into_bytes(), on_event).await.map(|_| ()).map_err(|f| (f.message, f.detail));
                if r.is_ok() {
                    *ctx.installed_desktop.borrow_mut() = Some(appimage_desktop_path(true, &p.id));
                }
                r
            }
            Kind::AppImage => {
                status.set_label(tr("Moving the app to its folder…"));
                let p2 = p.clone();
                match gio::spawn_blocking(move || install_appimage_user(&p2)).await.unwrap_or_else(|_| Err(String::new())) {
                    Ok(desktop) => {
                        log(&format!("{} → {}", p.path.display(), appimage_dir(false).display()));
                        log(&desktop.display().to_string());
                        *ctx.installed_desktop.borrow_mut() = Some(desktop);
                        Ok(())
                    }
                    Err(e) => Err((tr("The app could not be installed.").to_string(), e)),
                }
            }
            Kind::Flatpak | Kind::FlatpakRef => {
                status.set_label(tr("Installing the app with Flatpak…"));
                let mut argv: Vec<String> = ["flatpak", "install", if all { "--system" } else { "--user" }, "--noninteractive", "-y"].map(String::from).to_vec();
                argv.push(if p.kind == Kind::Flatpak { "--bundle".into() } else { "--from".into() });
                argv.push(p.path.to_string_lossy().into_owned());
                log(&argv.join(" "));
                run_streaming(&argv, |line| {
                    if let Some(f) = percent(line) {
                        bar.set_fraction(f.max(0.01));
                    } else {
                        log(line);
                    }
                })
                .await
                .map_err(|e| (tr("The app could not be installed.").to_string(), e))
            }
        };
        pulse.remove();
        ctx.busy.set(false);
        let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string();
        summary(&ctx, result.map_err(|(m, d)| (m, format!("{d}\n\n── {} ──\n{text}", tr("Log")))));
    });
}

/// Start the installed app, if there is something to start.
fn launcher(ctx: &C) -> Option<Rc<dyn Fn()>> {
    let p = ctx.pkg();
    match p.kind {
        Kind::Deb => {
            let id = p.desktop_id?;
            let app = gio::AppInfo::all().into_iter().find(|a| a.id().as_deref() == Some(id.as_str()))?;
            Some(Rc::new(move || {
                let _ = app.launch(&[], gio::AppLaunchContext::NONE);
            }))
        }
        Kind::Flatpak | Kind::FlatpakRef => Some(Rc::new(move || spawn_detached(&["flatpak", "run", &p.id]))),
        Kind::AppImage => {
            let desktop = ctx.installed_desktop.borrow().clone()?;
            Some(Rc::new(move || spawn_detached(&["gio", "launch", &desktop.to_string_lossy()])))
        }
    }
}

fn summary(ctx: &C, result: Result<(), (String, String)>) {
    let index = ctx.steps.borrow().len() - 1;
    ctx.current.set(index);
    render_steps(ctx);
    ctx.heading.set_label(tr("Summary"));
    ctx.back.set_visible(false);
    ctx.next.set_sensitive(true);
    ctx.next.set_label(tr("_Close"));
    let w = ctx.window.clone();
    *ctx.on_next.borrow_mut() = Some(Rc::new(move || w.close()));

    let b = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(12).valign(gtk::Align::Center).build();
    let icon = gtk::Image::builder().pixel_size(96).build();
    let (title, text) = match &result {
        Ok(()) => {
            icon.set_icon_name(Some("object-select-symbolic"));
            icon.add_css_class("success-icon");
            (tr("The Installation Was Successful").to_string(), tr("The app was installed.").to_string())
        }
        Err((message, _)) => {
            icon.set_icon_name(Some("dialog-error-symbolic"));
            icon.add_css_class("error-icon");
            (tr("The Installation Failed").to_string(), tr_dyn(message))
        }
    };
    b.append(&icon);
    b.append(&label(&title, &["title-2"]));
    b.append(&label(&text, &[]));
    ctx.content.set_child(Some(&b));

    match result {
        Ok(()) => {
            if let Some(open) = launcher(ctx) {
                ctx.extra.set_label(tr("_Open"));
                ctx.extra.set_visible(true);
                let w = ctx.window.clone();
                *ctx.on_extra.borrow_mut() = Some(Rc::new(move || {
                    open();
                    w.close();
                }));
            }
        }
        Err((_, details)) => {
            ctx.extra.set_label(tr("_Show Details"));
            ctx.extra.set_visible(true);
            let w = ctx.window.clone();
            *ctx.on_extra.borrow_mut() = Some(Rc::new(move || details_dialog(&w, tr("Installation Log"), &details)));
        }
    }
    ctx.next.grab_focus();
}

// ─────────────────────────── App ───────────────────────────

fn load_css() {
    crate::ui::load_css();
    let provider = gtk::CssProvider::new();
    provider.load_from_string(CSS);
    if let Some(display) = gtk::gdk::Display::default() {
        #[allow(deprecated)]
        gtk::style_context_add_provider_for_display(&display, &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }
}

fn temp_dir() -> PathBuf {
    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    glib::user_runtime_dir().join(format!("man-app-installer-{}-{n}", std::process::id()))
}

pub fn run(app: &adw::Application, path: PathBuf) {
    load_css();
    let window = adw::ApplicationWindow::builder().application(app).default_width(820).default_height(580).width_request(360).height_request(400).build();
    let title = adw::WindowTitle::new(tr("App Installer"), "");
    let header = adw::HeaderBar::builder().title_widget(&title).build();
    let tv = adw::ToolbarView::new();
    tv.add_top_bar(&header);

    let body = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let sidebar = gtk::Box::builder().orientation(gtk::Orientation::Vertical).width_request(230).build();
    sidebar.add_css_class("installer-sidebar");
    let step_list = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(16).margin_top(30).margin_start(28).margin_end(18).build();
    let side_icon = gtk::Box::builder().halign(gtk::Align::Center).valign(gtk::Align::End).vexpand(true).margin_bottom(30).build();
    sidebar.append(&step_list);
    sidebar.append(&side_icon);
    body.append(&sidebar);
    body.append(&gtk::Separator::new(gtk::Orientation::Vertical));

    let right = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(18).hexpand(true).margin_top(24).margin_bottom(18).margin_start(24).margin_end(24).build();
    let heading = gtk::Label::builder().xalign(0.0).wrap(true).build();
    heading.add_css_class("title-3");
    let content = adw::Bin::builder().vexpand(true).build();
    let buttons = gtk::Box::builder().spacing(12).halign(gtk::Align::End).build();
    let extra = gtk::Button::builder().use_underline(true).visible(false).build();
    let back = gtk::Button::builder().label(tr("Go _Back")).use_underline(true).width_request(120).build();
    let next = gtk::Button::builder().label(tr("_Continue")).use_underline(true).width_request(120).build();
    next.add_css_class("suggested-action");
    buttons.append(&extra);
    buttons.append(&back);
    buttons.append(&next);
    right.append(&heading);
    right.append(&content);
    right.append(&buttons);
    body.append(&right);
    tv.set_content(Some(&body));
    window.set_content(Some(&tv));

    // Narrow windows: the steps make room for the content.
    let bp = adw::Breakpoint::new(adw::BreakpointCondition::parse("max-width: 600sp").expect("condition"));
    bp.add_setter(&sidebar, "visible", Some(&false.to_value()));
    window.add_breakpoint(bp);

    let temp = temp_dir();
    let ctx: C = Rc::new(Ctx {
        window: window.clone(),
        title,
        steps: RefCell::new(vec![]),
        current: Cell::new(0),
        step_list,
        side_icon,
        heading,
        content,
        back: back.clone(),
        next: next.clone(),
        extra: extra.clone(),
        on_next: RefCell::new(None),
        on_extra: RefCell::new(None),
        pkg: RefCell::new(Package::default()),
        all_users: Cell::new(false),
        busy: Cell::new(false),
        deb_ok: Cell::new(false),
        installed_desktop: RefCell::new(None),
    });
    {
        let c = ctx.clone();
        next.connect_clicked(move |_| {
            let f = c.on_next.borrow().clone();
            if let Some(f) = f {
                f();
            }
        });
        let c = ctx.clone();
        extra.connect_clicked(move |_| {
            let f = c.on_extra.borrow().clone();
            if let Some(f) = f {
                f();
            }
        });
        let c = ctx.clone();
        back.connect_clicked(move |_| {
            let i = c.current.get();
            if i > 0 {
                show(&c, i - 1);
            }
        });
        let (c, temp) = (ctx.clone(), temp.clone());
        window.connect_close_request(move |_| {
            if c.busy.get() {
                return glib::Propagation::Stop;
            }
            let _ = std::fs::remove_dir_all(&temp);
            glib::Propagation::Proceed
        });
    }

    // Reading the file can take a moment (large packages, AppImages).
    ctx.heading.set_label(tr("Reading the app…"));
    let spinner = adw::Spinner::builder().width_request(48).height_request(48).halign(gtk::Align::Center).valign(gtk::Align::Center).build();
    ctx.content.set_child(Some(&spinner));
    back.set_sensitive(false);
    next.set_sensitive(false);
    set_side_icon(&ctx);
    window.present();
    glib::spawn_future_local(async move {
        let pkg = gio::spawn_blocking(move || read_package(path, temp)).await.unwrap_or_default();
        ctx.title.set_title(&trf("Install {name}", &[("name", &pkg.name)]));
        ctx.window.set_title(Some(&trf("Install {name}", &[("name", &pkg.name)])));
        let mut steps = vec![Step::Introduction];
        if !pkg.license_text.trim().is_empty() {
            steps.push(Step::License);
        }
        if pkg.kind != Kind::Deb {
            steps.push(Step::Destination);
        }
        steps.extend([Step::Type, Step::Installation, Step::Summary]);
        *ctx.steps.borrow_mut() = steps;
        *ctx.pkg.borrow_mut() = pkg;
        set_side_icon(&ctx);
        show(&ctx, 0);
    });
}

fn choose_file(app: &adw::Application) {
    let filter = gtk::FileFilter::new();
    filter.set_name(Some(tr("Apps")));
    for pattern in ["*.deb", "*.flatpak", "*.flatpakref", "*.AppImage", "*.appimage"] {
        filter.add_pattern(pattern);
    }
    let filters = gio::ListStore::new::<gtk::FileFilter>();
    filters.append(&filter);
    let dialog = gtk::FileDialog::builder().title(tr("Choose an App to Install")).filters(&filters).build();
    if let Some(downloads) = glib::user_special_dir(glib::UserDirectory::Downloads) {
        dialog.set_initial_folder(Some(&gio::File::for_path(downloads)));
    }
    let app = app.clone();
    let hold = app.hold();
    dialog.open(gtk::Window::NONE, gio::Cancellable::NONE, move |res| {
        drop(hold);
        if let Some(path) = res.ok().and_then(|f| f.path()) {
            run(&app, path);
        }
    });
}

pub fn main() -> glib::ExitCode {
    crate::i18n::init();
    let app = adw::Application::builder().application_id(APP_ID).flags(gio::ApplicationFlags::HANDLES_OPEN | gio::ApplicationFlags::NON_UNIQUE).build();
    app.connect_open(|app, files, _| {
        for path in files.iter().filter_map(|f| f.path()) {
            run(app, path);
        }
    });
    app.connect_activate(|app| {
        if app.active_window().is_none() {
            choose_file(app);
        }
    });
    app.run()
}
