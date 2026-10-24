//! Languages, formats and time zones: locale generation on demand, system
//! and account languages.
use crate::util::*;
use anyhow::{bail, Result};
use man_common::*;
use std::path::Path;

/// glibc normalizes "cs_CZ.UTF-8" to "cs_CZ.utf8" in `locale -a`.
fn normalized(locale: &str) -> String {
    let (base, modifier) = locale.split_once('@').map(|(b, m)| (b, Some(m))).unwrap_or((locale, None));
    let base = base.replace(".UTF-8", ".utf8");
    match modifier {
        Some(m) => format!("{base}@{m}"),
        None => base,
    }
}

fn in_root(root: &Path, args: &[&str]) -> Result<String> {
    if root == Path::new("/") {
        run(args[0], &args[1..])
    } else {
        chroot(root, args)
    }
}

/// Generate the locale (and keep it in /etc/locale.gen) if it is missing.
pub fn ensure_locale(root: &Path, locale: &str) -> Result<()> {
    if !valid_locale(locale) {
        bail!("Unknown language {locale}.");
    }
    let have = in_root(root, &["locale", "-a"]).unwrap_or_default();
    let norm = normalized(locale);
    if have.lines().any(|l| l.trim() == norm) {
        return Ok(());
    }
    // /etc/locale.gen lists "cs_CZ.UTF-8 UTF-8"; make sure it is enabled so
    // a later locale-gen keeps it.
    let gen = root.join("etc/locale.gen");
    let entry = format!("{locale} UTF-8");
    let text = read_to_string(&gen).unwrap_or_default();
    let mut found = false;
    let mut out: Vec<String> = text
        .lines()
        .map(|l| {
            if l.trim_start_matches('#').trim() == entry {
                found = true;
                entry.clone()
            } else {
                l.to_string()
            }
        })
        .collect();
    if !found {
        out.push(entry);
    }
    write_file(&gen, &(out.join("\n") + "\n"), 0o644)?;
    // localedef -i cs_CZ -f UTF-8 cs_CZ.UTF-8 (modifier goes on the input).
    let (base, modifier) = locale.split_once('@').unwrap_or((locale, ""));
    let mut input = base.trim_end_matches(".UTF-8").to_string();
    if !modifier.is_empty() {
        input = format!("{input}@{modifier}");
    }
    info(format!("Generating the {locale} locale"));
    in_root(root, &["localedef", "-i", &input, "-f", "UTF-8", locale])?;
    Ok(())
}

/// The locale used for dates, numbers and units in `country`: the
/// language's own variant when it exists ("de" + AT → de_AT), otherwise the
/// country's main locale, otherwise the language itself.
pub fn formats_locale(language: &str, country: &str) -> String {
    if !valid_country(country) {
        return language.to_string();
    }
    let utf8 = supported_utf8();
    let ll = language.split(['_', '.']).next().unwrap_or("en");
    let own = format!("{ll}_{country}.UTF-8");
    if utf8.contains(&own) {
        return own;
    }
    let marker = format!("_{country}.UTF-8");
    utf8.into_iter().find(|l| l.ends_with(&marker)).unwrap_or_else(|| language.to_string())
}

/// UTF-8 locales glibc can generate, as "ll_CC.UTF-8[@mod]". SUPPORTED lists
/// them as "cs_CZ.UTF-8 UTF-8", or "ta_IN UTF-8" when UTF-8 is the default.
fn supported_utf8() -> Vec<String> {
    let text = read_to_string("/usr/share/i18n/SUPPORTED").unwrap_or_default();
    text.lines()
        .filter_map(|l| l.strip_suffix(" UTF-8"))
        .map(|name| {
            let (base, modifier) = name.split_once('@').map(|(b, m)| (b, Some(m))).unwrap_or((name, None));
            let base = base.trim_end_matches(".UTF-8");
            match modifier {
                Some(m) => format!("{base}.UTF-8@{m}"),
                None => format!("{base}.UTF-8"),
            }
        })
        .filter(|l| valid_locale(l))
        .collect()
}

/// Main time zone of a country, from tzdata's zone.tab.
pub fn country_timezone(country: &str) -> Option<String> {
    let tab = read_to_string("/usr/share/zoneinfo/zone.tab")?;
    tab.lines().filter(|l| !l.starts_with('#')).find_map(|l| {
        let f: Vec<&str> = l.split('\t').collect();
        (f.first() == Some(&country) && f.len() > 2).then(|| f[2].to_string())
    })
}

pub fn current_timezone(root: &Path) -> String {
    std::fs::read_link(root.join("etc/localtime"))
        .ok()
        .and_then(|p| p.to_str().and_then(|s| s.split("zoneinfo/").nth(1).map(String::from)))
        .unwrap_or_default()
}

pub fn set_timezone(root: &Path, tz: &str) -> Result<()> {
    if tz.contains("..") || !Path::new("/usr/share/zoneinfo").join(tz).is_file() {
        bail!("Unknown time zone {tz}.");
    }
    let lt = root.join("etc/localtime");
    let _ = std::fs::remove_file(&lt);
    std::os::unix::fs::symlink(format!("/usr/share/zoneinfo/{tz}"), &lt)?;
    write_file(root.join("etc/timezone"), &format!("{tz}\n"), 0o644)?;
    Ok(())
}

/// System language: GDM, the console and new accounts.
pub fn set_system_language(root: &Path, locale: &str) -> Result<()> {
    ensure_locale(root, locale)?;
    let text = format!("LANG={locale}\n");
    write_file(root.join("etc/default/locale"), &text, 0o644)?;
    let conf = root.join("etc/locale.conf");
    if !conf.is_symlink() {
        write_file(conf, &text, 0o644)?;
    }
    Ok(())
}

/// Language and formats of one account (AccountsService; GDM starts the
/// session with them).
pub fn set_user_language(root: &Path, user: &str, locale: &str, formats: Option<&str>) -> Result<()> {
    let path = root.join("var/lib/AccountsService/users").join(user);
    let text = read_to_string(&path).unwrap_or_else(|| "[User]\n".into());
    let mut out: Vec<String> = text.lines().filter(|l| !l.starts_with("Language=") && !l.starts_with("FormatsLocale=")).map(String::from).collect();
    if !out.iter().any(|l| l.trim() == "[User]") {
        out.insert(0, "[User]".into());
    }
    let at = out.iter().position(|l| l.trim() == "[User]").unwrap() + 1;
    if let Some(f) = formats {
        out.insert(at, format!("FormatsLocale={f}"));
    }
    out.insert(at, format!("Language={locale}"));
    write_file(&path, &(out.join("\n") + "\n"), 0o600)?;
    // A running accounts-daemon caches users: tell it too.
    if root == Path::new("/") {
        if let Some(uid) = query("id", &["-u", user]) {
            let obj = format!("/org/freedesktop/Accounts/User{uid}");
            let _ = query("busctl", &["call", "org.freedesktop.Accounts", &obj, "org.freedesktop.Accounts.User", "SetLanguage", "s", locale]);
            if let Some(f) = formats {
                let _ = query("busctl", &["call", "org.freedesktop.Accounts", &obj, "org.freedesktop.Accounts.User", "SetFormatsLocale", "s", f]);
            }
        }
    }
    Ok(())
}

pub fn current_language(root: &Path) -> Option<String> {
    read_to_string(root.join("etc/default/locale"))?
        .lines()
        .find_map(|l| l.strip_prefix("LANG=").map(|v| v.trim_matches('"').to_string()))
        .filter(|l| valid_locale(l))
}

/// Live session: the language MANInstaller logs in with next time.
pub fn set_live_language(locale: &str) -> Result<String> {
    if query("id", &["-u"]).as_deref() != Some("0") {
        bail!("The language must be changed by the system helper.");
    }
    if language(locale).is_none() {
        bail!("Unknown language {locale}.");
    }
    set_system_language(Path::new("/"), locale)?;
    set_user_language(Path::new("/"), "MANInstaller", locale, None)?;
    // Skip the "Update standard folders?" question in the live session:
    // xdg-user-dirs-gtk asks when this differs from the session language.
    let dirs_locale = Path::new("/home/MANInstaller/.config/user-dirs.locale");
    if dirs_locale.parent().is_some_and(|p| p.is_dir()) {
        let _ = write_file(dirs_locale, locale.split('.').next().unwrap_or(locale), 0o644);
        let _ = query("chown", &["MANInstaller:MANInstaller", &dirs_locale.to_string_lossy()]);
    }
    // New session in the new language: GDM logs MANInstaller in again
    // automatically when it starts. Run it outside this helper's process.
    run("systemd-run", &["--on-active=2", "--unit=man-live-relogin", "--collect", "systemctl", "restart", "display-manager.service"])?;
    Ok(format!("Language set to {}.", language(locale).map(|l| l.native).unwrap_or(locale)))
}
