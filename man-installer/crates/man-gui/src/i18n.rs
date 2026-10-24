//! Translations of MAN's own apps. Catalogs are standard gettext `.mo` files
//! (`/usr/share/locale/<lang>/LC_MESSAGES/man-installer.mo`, built from
//! `po/<lang>.po`). The language follows the session, and MAN Setup switches it
//! while running: pages built after `set_language` use the new language.
use std::collections::HashMap;
use std::sync::RwLock;

pub const DOMAIN: &str = "man-installer";
const LOCALE_DIR: &str = "/usr/share/locale";

type Catalog = HashMap<String, &'static str>;

static CATALOG: RwLock<Option<&'static Catalog>> = RwLock::new(None);
static LANGUAGE: RwLock<String> = RwLock::new(String::new());

/// Marks a message for translation where `tr` cannot run (constants);
/// translate it with `tr` where it is shown.
pub const fn n_(msgid: &'static str) -> &'static str {
    msgid
}

/// Translate a message written in the source.
pub fn tr(msgid: &'static str) -> &'static str {
    lookup(msgid).unwrap_or(msgid)
}

/// Translate a message known only at run time, e.g. one sent by the system
/// helper. Unknown messages are returned unchanged.
pub fn tr_dyn(msgid: &str) -> String {
    lookup(msgid).map(str::to_string).unwrap_or_else(|| msgid.to_string())
}

/// Translate, then fill `{name}` placeholders.
pub fn trf(msgid: &'static str, args: &[(&str, &dyn std::fmt::Display)]) -> String {
    fill(tr(msgid), args)
}

pub fn fill(template: &str, args: &[(&str, &dyn std::fmt::Display)]) -> String {
    let mut out = template.to_string();
    for (name, value) in args {
        out = out.replace(&format!("{{{name}}}"), &value.to_string());
    }
    out
}

fn lookup(msgid: &str) -> Option<&'static str> {
    let cat = *CATALOG.read().ok()?;
    cat?.get(msgid).copied().filter(|s| !s.is_empty())
}

/// Locale the apps are shown in, e.g. "cs_CZ.UTF-8" (empty: English).
pub fn language() -> String {
    LANGUAGE.read().map(|l| l.clone()).unwrap_or_default()
}

/// The session's language from the environment, as gettext chooses it.
fn session_language() -> String {
    for var in ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(var) {
            let first = v.split(':').next().unwrap_or("").trim().to_string();
            if !first.is_empty() {
                return first;
            }
        }
    }
    String::new()
}

/// Catalog names to try for a locale: "pt_BR.UTF-8" → pt_BR, pt.
fn candidates(locale: &str) -> Vec<String> {
    let base = locale.split(['.', '@']).next().unwrap_or("");
    let mut v = vec![base.to_string()];
    if let Some((ll, _)) = base.split_once('_') {
        v.push(ll.to_string());
    }
    v.retain(|s| !s.is_empty() && s != "C" && s != "POSIX");
    v
}

/// Load the catalog of `domain` for `locale` from the locale directory.
pub fn load_catalog(domain: &str, locale: &str) -> Option<HashMap<String, String>> {
    candidates(locale).iter().find_map(|c| {
        let path = format!("{LOCALE_DIR}/{c}/LC_MESSAGES/{domain}.mo");
        std::fs::read(path).ok().and_then(|data| parse_mo(&data))
    })
}

/// Use the session's language. Call once at start.
pub fn init() {
    set_language(&session_language());
}

/// Switch the apps' language; widgets built afterwards use it.
pub fn set_language(locale: &str) {
    let cat = load_catalog(DOMAIN, locale).map(|c| {
        let leaked: Catalog = c.into_iter().map(|(k, v)| (k, &*Box::leak(v.into_boxed_str()))).collect();
        &*Box::leak(Box::new(leaked))
    });
    if let Ok(mut c) = CATALOG.write() {
        *c = cat;
    }
    if let Ok(mut l) = LANGUAGE.write() {
        *l = locale.to_string();
    }
    let rtl = matches!(candidates(locale).last().map(String::as_str), Some("ar" | "he" | "fa" | "ur" | "yi"));
    if gtk::is_initialized_main_thread() {
        gtk::Widget::set_default_direction(if rtl { gtk::TextDirection::Rtl } else { gtk::TextDirection::Ltr });
    }
}

/// Parse a gettext `.mo` file (either byte order). Contexts and plural forms
/// are kept as gettext stores them; MAN's catalogs use neither.
pub fn parse_mo(data: &[u8]) -> Option<HashMap<String, String>> {
    let word = |at: usize, le: bool| -> Option<usize> {
        let b: [u8; 4] = data.get(at..at + 4)?.try_into().ok()?;
        Some(if le { u32::from_le_bytes(b) } else { u32::from_be_bytes(b) } as usize)
    };
    let le = match word(0, true)? {
        0x9504_12de => true,
        0xde12_0495 => false,
        _ => return None,
    };
    let n = word(8, le)?;
    let originals = word(12, le)?;
    let translations = word(16, le)?;
    let string = |table: usize, i: usize| -> Option<String> {
        let len = word(table + i * 8, le)?;
        let off = word(table + i * 8 + 4, le)?;
        String::from_utf8(data.get(off..off + len)?.to_vec()).ok()
    };
    let mut map = HashMap::with_capacity(n);
    for i in 0..n {
        let (k, v) = (string(originals, i)?, string(translations, i)?);
        if !k.is_empty() {
            map.insert(k, v);
        }
    }
    Some(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A two-entry little-endian .mo, as msgfmt writes it.
    fn sample() -> Vec<u8> {
        let entries = [("", "Content-Type: text/plain; charset=UTF-8\n"), ("Next", "Další")];
        let n = entries.len();
        let mut out = vec![];
        let header = 28;
        let otab = header;
        let ttab = otab + n * 8;
        let mut strings = vec![];
        let mut pos = ttab + n * 8;
        let mut o = vec![];
        let mut t = vec![];
        for (k, _) in &entries {
            o.push((k.len(), pos));
            strings.extend_from_slice(k.as_bytes());
            strings.push(0);
            pos += k.len() + 1;
        }
        for (_, v) in &entries {
            t.push((v.len(), pos));
            strings.extend_from_slice(v.as_bytes());
            strings.push(0);
            pos += v.len() + 1;
        }
        for x in [0x9504_12deu32, 0, n as u32, otab as u32, ttab as u32, 0, 0] {
            out.extend_from_slice(&x.to_le_bytes());
        }
        for (l, p) in o.iter().chain(t.iter()) {
            out.extend_from_slice(&(*l as u32).to_le_bytes());
            out.extend_from_slice(&(*p as u32).to_le_bytes());
        }
        out.extend_from_slice(&strings);
        out
    }

    #[test]
    fn reads_mo() {
        let m = parse_mo(&sample()).unwrap();
        assert_eq!(m.get("Next").map(String::as_str), Some("Další"));
        assert!(!m.contains_key(""));
        assert!(parse_mo(b"nonsense").is_none());
    }

    #[test]
    fn candidates_and_fill() {
        assert_eq!(candidates("pt_BR.UTF-8"), vec!["pt_BR", "pt"]);
        assert_eq!(candidates("sr_RS.UTF-8@latin"), vec!["sr_RS", "sr"]);
        assert!(candidates("C.UTF-8").is_empty());
        assert_eq!(fill("{a} of {b}", &[("a", &1), ("b", &"two")]), "1 of two");
    }
}
