//! MAN Setup (OOBE): finishing the first start, and disk encryption at the
//! following start.
use crate::install::{create_user, gdm_config, set_root_password};
use crate::util::*;
use anyhow::{anyhow, bail, Context, Result};
use man_common::*;
use std::path::Path;

const OOBE_CLEANUP: &str = "/var/lib/man/oobe-cleanup";
const ENCRYPT_KEY: &str = "/var/lib/man/encrypt.key";
/// Space freed at the end of the partition for the LUKS2 header.
const LUKS_SPACE: u64 = 32 << 20;

fn step(i: u32, total: u32, title: &str) {
    emit(&Event::Step { index: i, total, title: title.into() });
}

#[derive(Debug, PartialEq)]
pub enum EncryptionState {
    Encrypted,
    Available { device: String },
    Unavailable(String),
}

fn mount_source(target: &str) -> Option<String> {
    query("findmnt", &["-n", "-o", "SOURCE", "--mountpoint", target]).map(|s| s.split('[').next().unwrap_or("").to_string())
}

pub fn encryption_state() -> EncryptionState {
    let Some(root) = mount_source("/") else { return EncryptionState::Unavailable("The system disk was not found.".into()) };
    if root.starts_with("/dev/mapper/") || root.starts_with("/dev/dm-") {
        return EncryptionState::Encrypted;
    }
    if query("findmnt", &["-n", "-o", "FSTYPE", "--mountpoint", "/"]).as_deref() != Some("btrfs") {
        return EncryptionState::Unavailable("Encryption needs the Btrfs layout created by the MAN installer.".into());
    }
    let boot_label = mount_source("/boot").and_then(|b| query("blkid", &["-s", "LABEL", "-o", "value", &b]));
    if boot_label.as_deref() != Some("MAN_BOOT") {
        return EncryptionState::Unavailable("This installation has no separate boot partition, which encryption needs.".into());
    }
    EncryptionState::Available { device: root }
}

fn write_hostname(name: &str) -> Result<()> {
    write_file("/etc/hostname", &format!("{name}\n"), 0o644)?;
    write_file(
        "/etc/hosts",
        &format!("127.0.0.1 localhost\n127.0.1.1 {name}\n\n::1 localhost ip6-localhost ip6-loopback\nff02::1 ip6-allnodes\nff02::2 ip6-allrouters\n"),
        0o644,
    )?;
    let _ = run("hostname", &[name]);
    Ok(())
}

fn gvariant_str(s: &str) -> String {
    format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'"))
}

/// The user's GNOME settings, in `dconf load` keyfile format.
fn user_settings(c: &OobeConfig) -> String {
    let a = &c.appearance;
    let uri = |p: &str| gvariant_str(&format!("file://{p}"));
    let scheme = if a.color_scheme == "prefer-dark" { "prefer-dark" } else { "default" };
    let mut s = String::new();
    s += &format!("[org/gnome/desktop/interface]\ncolor-scheme={}\naccent-color={}\n\n", gvariant_str(scheme), gvariant_str(&a.accent));
    if !a.wallpaper.is_empty() {
        let dark = if a.wallpaper_dark.is_empty() { &a.wallpaper } else { &a.wallpaper_dark };
        s += &format!(
            "[org/gnome/desktop/background]\npicture-uri={}\npicture-uri-dark={}\npicture-options='zoom'\n\n[org/gnome/desktop/screensaver]\npicture-uri={}\npicture-uri-dark={}\n\n",
            uri(&a.wallpaper), uri(dark), uri(&a.wallpaper), uri(dark)
        );
    }
    s += &format!("[org/gnome/system/location]\nenabled={}\n\n", c.location);
    s += &format!("[org/gnome/desktop/datetime]\nautomatic-timezone={}\n", c.location && c.automatic_timezone);
    if !c.language.is_empty() {
        let formats = crate::locale::formats_locale(&c.language, &c.country);
        s += &format!("\n[org/gnome/system/locale]\nregion={}\n", gvariant_str(&formats));
    }
    s
}

/// Write the user's dconf database directly (no D-Bus session needed): the
/// same binary file GNOME reads at login.
fn apply_user_settings(c: &OobeConfig) -> Result<()> {
    let u = &c.user.username;
    let dir = format!("/run/man-oobe-dconf-{u}");
    let _ = std::fs::remove_dir_all(&dir);
    write_file(format!("{dir}/00-man-setup"), &user_settings(c), 0o644)?;
    let db_dir = format!("/home/{u}/.config/dconf");
    std::fs::create_dir_all(&db_dir)?;
    let res = run("dconf", &["compile", &format!("{db_dir}/user"), &dir]).context("could not save your desktop settings");
    let _ = std::fs::remove_dir_all(&dir);
    res?;
    run("chown", &["-R", &format!("{u}:{u}"), &format!("/home/{u}/.config")])?;
    Ok(())
}

fn validate(c: &OobeConfig) -> Result<()> {
    if query("id", &["-u"]).as_deref() != Some("0") {
        bail!("MAN Setup must be finished by the system helper.");
    }
    if !Path::new(OOBE_PENDING).exists() {
        bail!("MAN Setup has already been completed on this computer.");
    }
    if !valid_username(&c.user.username) {
        bail!("The username is not valid.");
    }
    if query("id", &["-u", &c.user.username]).is_some() {
        bail!("The username “{}” is already taken.", c.user.username);
    }
    if c.user.password.is_empty() {
        bail!("A password is required.");
    }
    if !c.language.is_empty() && language(&c.language).is_none() {
        bail!("Unknown language {}.", c.language);
    }
    if !c.country.is_empty() && !valid_country(&c.country) {
        bail!("Unknown country {}.", c.country);
    }
    if !valid_hostname(&c.hostname) {
        bail!("The computer name is not valid.");
    }
    if !ACCENTS.iter().any(|a| a.0 == c.appearance.accent) {
        bail!("Unknown accent color.");
    }
    for w in [&c.appearance.wallpaper, &c.appearance.wallpaper_dark] {
        if !w.is_empty() && (!w.starts_with("/usr/share/backgrounds/") || w.contains("..") || !Path::new(w).is_file()) {
            bail!("The chosen wallpaper is not available.");
        }
    }
    if let Some(p) = &c.encrypt_passphrase {
        if p.chars().count() < 8 {
            bail!("The encryption passphrase must have at least 8 characters.");
        }
        if let EncryptionState::Unavailable(why) = encryption_state() {
            bail!("{why}");
        }
    }
    Ok(())
}

/// Shrink Btrfs to leave room for the LUKS2 header, and stage everything the
/// initramfs needs to encrypt the partition at the next start.
fn prepare_encryption(passphrase: &str) -> Result<()> {
    let EncryptionState::Available { device } = encryption_state() else {
        bail!("This system cannot be encrypted.");
    };
    let partuuid = query("blkid", &["-s", "PARTUUID", "-o", "value", &device]).filter(|s| !s.is_empty()).ok_or_else(|| anyhow!("{device} has no partition UUID"))?;
    let part_size: u64 = run("blockdev", &["--getsize64", &device])?.trim().parse()?;
    let fs_size: u64 = run("btrfs", &["filesystem", "show", "--raw", "/"])?
        .lines()
        .find_map(|l| {
            let w: Vec<&str> = l.split_whitespace().collect();
            (w.first() == Some(&"devid") && w.len() > 3 && w[2] == "size").then(|| w[3].parse().ok()).flatten()
        })
        .ok_or_else(|| anyhow!("cannot read the Btrfs size"))?;
    let target = (part_size - LUKS_SPACE) & !4095;
    if fs_size > target {
        run("btrfs", &["filesystem", "resize", &format!("1:{target}"), "/"])?;
    }
    let luks_uuid = read_to_string("/proc/sys/kernel/random/uuid").ok_or_else(|| anyhow!("no UUID source"))?.trim().to_string();
    // The key lives on the still-unencrypted system disk; encrypting the
    // partition in place encrypts its old blocks too, and it is deleted on
    // the first start after encryption.
    write_file(ENCRYPT_KEY, passphrase, 0o600)?;
    write_file(ENCRYPT_PENDING, &format!("DEVICE=/dev/disk/by-partuuid/{partuuid}\nLUKS_UUID={luks_uuid}\n"), 0o644)?;
    write_file("/etc/crypttab", &format!("# <name> <device> <key> <options>\nman_root UUID={luks_uuid} none luks,discard\n"), 0o600)?;
    let hook = "/etc/cryptsetup-initramfs/conf-hook";
    let text = read_to_string(hook).unwrap_or_default();
    let text = if text.lines().any(|l| l.trim() == "CRYPTSETUP=y") { text } else { format!("{}CRYPTSETUP=y\n", text) };
    write_file(hook, &text, 0o644)?;
    run("update-initramfs", &["-u", "-k", "all"])?;
    Ok(())
}

/// Language for the system and the new account, formats of the country, and
/// the country's time zone when the installer left it at UTC.
fn apply_language(c: &OobeConfig) -> Result<()> {
    use crate::locale::*;
    if c.language.is_empty() {
        return Ok(());
    }
    let root = Path::new("/");
    set_system_language(root, &c.language)?;
    let formats = formats_locale(&c.language, &c.country);
    ensure_locale(root, &formats)?;
    set_user_language(root, &c.user.username, &c.language, Some(&formats))?;
    if matches!(current_timezone(root).as_str(), "" | "Etc/UTC" | "UTC" | "Etc/Universal") {
        if let Some(tz) = country_timezone(&c.country) {
            set_timezone(root, &tz)?;
        }
    }
    Ok(())
}

pub fn apply(c: OobeConfig) -> Result<String> {
    let total = if c.encrypt_passphrase.is_some() { 4 } else { 3 };
    step(1, total, "Creating your account");
    validate(&c)?;
    write_hostname(&c.hostname)?;
    create_user(Path::new("/"), &c.user)?;
    set_root_password(Path::new("/"), c.root_password.as_deref())?;

    step(2, total, "Applying your settings");
    apply_user_settings(&c)?;
    apply_language(&c)?;
    let gdm = "/etc/gdm3/daemon.conf";
    let text = read_to_string(gdm).unwrap_or_default();
    write_file(gdm, &gdm_config(&text, c.user.autologin.then_some(c.user.username.as_str()), Some("gnome")), 0o644)?;

    if let Some(p) = &c.encrypt_passphrase {
        step(3, total, "Preparing disk encryption");
        prepare_encryption(p)?;
    }

    step(total, total, "Finishing");
    // The setup account is in use until the restart; remove it then.
    write_file(OOBE_CLEANUP, "", 0o644)?;
    std::fs::remove_file(OOBE_PENDING)?;
    let _ = run("sync", &[]);
    Ok(if c.encrypt_passphrase.is_some() {
        "MAN is ready. After the restart, your disk is encrypted in the background while you use the computer.".into()
    } else {
        "MAN is ready.".into()
    })
}

/// At boot, before the login screen: remove the setup account.
pub fn cleanup() -> Result<()> {
    if !Path::new(OOBE_CLEANUP).exists() {
        return Ok(());
    }
    if query("id", &["-u", OOBE_USER]).is_some() {
        run("userdel", &["-r", "-f", OOBE_USER])?;
    }
    let _ = std::fs::remove_file(format!("/var/lib/AccountsService/users/{OOBE_USER}"));
    std::fs::remove_file(OOBE_CLEANUP)?;
    Ok(())
}

fn pending_device() -> Option<String> {
    read_to_string(ENCRYPT_PENDING)?.lines().find_map(|l| l.strip_prefix("DEVICE=").map(|d| d.trim().to_string()))
}

/// Background part of disk encryption (man-encrypt.service). The initramfs
/// started LUKS2 encryption and unlocked the disk; finish it while MAN runs.
/// cryptsetup records its progress, so an interrupted run simply resumes.
pub fn encrypt_online() -> Result<()> {
    let Some(device) = pending_device() else { return Ok(()) };
    if encryption_state() != EncryptionState::Encrypted {
        info("Disk encryption has not started yet; it starts at the next restart.");
        return Ok(());
    }
    let dump = run("cryptsetup", &["luksDump", &device])?;
    if dump.contains("online-reencrypt") || dump.contains("reencrypt") {
        if !Path::new(ENCRYPT_KEY).exists() {
            bail!("The setup key is missing, so encryption cannot continue in the background.");
        }
        info("Encrypting the system disk in the background…");
        let status = std::process::Command::new("cryptsetup")
            .args(["reencrypt", "--batch-mode", "--resume-only", "--key-file", ENCRYPT_KEY, "--progress-frequency", "30", &device])
            .status()
            .context("cannot start cryptsetup")?;
        if !status.success() {
            bail!("cryptsetup stopped ({status}); encryption will continue later");
        }
    }
    if Path::new(ENCRYPT_KEY).exists() {
        let _ = run("shred", &["-u", "-z", ENCRYPT_KEY]);
        let _ = std::fs::remove_file(ENCRYPT_KEY);
    }
    std::fs::remove_file(ENCRYPT_PENDING)?;
    // The boot image no longer needs the MAN encryption step.
    run("update-initramfs", &["-u", "-k", "all"])?;
    info("Disk encryption completed.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_keyfile() {
        let c = OobeConfig {
            appearance: Appearance {
                color_scheme: "prefer-dark".into(),
                accent: "teal".into(),
                wallpaper: "/usr/share/backgrounds/gnome/a-l.jxl".into(),
                wallpaper_dark: "/usr/share/backgrounds/gnome/a-d.jxl".into(),
            },
            location: false,
            automatic_timezone: true,
            ..Default::default()
        };
        let s = user_settings(&c);
        assert!(s.contains("color-scheme='prefer-dark'"));
        assert!(s.contains("accent-color='teal'"));
        assert!(s.contains("picture-uri-dark='file:///usr/share/backgrounds/gnome/a-d.jxl'"));
        assert!(s.contains("[org/gnome/system/location]\nenabled=false"));
        assert!(s.contains("automatic-timezone=false"));
        assert_eq!(gvariant_str("it's"), "'it\\'s'");
    }
}
