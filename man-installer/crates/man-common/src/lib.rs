//! Types shared by the MAN installer GUI, the snapshot manager and the
//! privileged backend. The GUI talks to the backend over pkexec: requests are
//! one JSON document on stdin, replies are JSON lines ([`Event`]) on stdout.

use serde::{Deserialize, Serialize};

mod languages;
pub use languages::*;

/// Privileged helper, started through pkexec (see the polkit policy).
pub const BACKEND_PATH: &str = "/usr/libexec/man-installer/man-installer-backend";
/// Where the `@snapshots` subvolume is mounted on an installed system.
pub const SNAPSHOT_DIR: &str = "/.snapshots";
/// Snapshot settings, written by the backend, read by everyone.
pub const SNAPSHOT_CONFIG: &str = "/etc/man/snapshots.conf";
/// Btrfs filesystem label used for MAN system partitions.
pub const BTRFS_LABEL: &str = "MAN";
/// File system label of the live medium's boot partition (the "USB stick").
pub const LIVE_LABEL: &str = "MAN_LIVE";
/// LUKS2 header label used for encrypted MAN system partitions.
pub const LUKS_LABEL: &str = "MAN_CRYPT";
/// Space the installer keeps free on top of the copied system.
pub const HEADROOM: u64 = 4 << 30;
/// Disks smaller than this are not offered at all.
pub const MIN_DISK: u64 = 16 << 30;

pub const ESP_GUID: &str = "c12a7328-f81f-11d2-ba4b-00a0c93ec93b";
/// Microsoft reserved partition, present on every GPT Windows disk.
pub const MSR_GUID: &str = "e3c9e316-0b5c-4db8-817d-f92df00215ae";
/// Size of the separate /boot partition the installer creates.
pub const BOOT_SIZE: u64 = 1 << 30;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Probe {
    pub uefi: bool,
    /// Disk the running system lives on; never offered as a target.
    pub live_disk: Option<String>,
    pub disks: Vec<Disk>,
    pub installs: Vec<ExistingInstall>,
    /// Bytes the installer will copy.
    pub source_bytes: u64,
    pub timezone: String,
    pub keyboard: String,
    /// Language (locale) of the running system.
    #[serde(default)]
    pub language: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Disk {
    pub path: String,
    pub model: String,
    pub serial: String,
    pub size: u64,
    pub transport: String,
    pub removable: bool,
    pub read_only: bool,
    pub is_live: bool,
    /// Some partition of this disk is mounted or used as swap.
    pub busy: bool,
    pub partitions: Vec<Partition>,
    /// Unpartitioned space (aligned to 1 MiB, at least 1 GiB), largest first.
    #[serde(default)]
    pub free: Vec<FreeSpace>,
    /// Windows is installed on this disk (NTFS or Microsoft reserved partition).
    #[serde(default)]
    pub windows: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FreeSpace {
    /// Offset and size in bytes.
    pub start: u64,
    pub size: u64,
}

impl Disk {
    pub fn largest_free(&self) -> Option<FreeSpace> {
        self.free.iter().copied().max_by_key(|f| f.size)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Partition {
    pub path: String,
    pub disk: String,
    pub size: u64,
    pub fstype: Option<String>,
    pub label: Option<String>,
    pub uuid: Option<String>,
    pub parttype: Option<String>,
    pub mountpoints: Vec<String>,
    pub is_esp: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ExistingInstall {
    pub disk: String,
    /// Btrfs partition, or the LUKS container holding it.
    pub device: String,
    pub encrypted: bool,
    /// False while an encrypted install has not been unlocked yet.
    pub inspected: bool,
    pub has_system: bool,
    pub has_home: bool,
    pub home_dirs: Vec<String>,
    pub esp: Option<String>,
    pub boot: Option<String>,
    pub snapshots: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Repartition a whole disk.
    #[default]
    Erase,
    /// Use partitions the user picked.
    Manual,
    /// Replace @system of an existing MAN install, keep @home.
    Reinstall,
    /// Use the free space next to Windows (made by DUHACH!), keep Windows.
    Alongside,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct User {
    pub full_name: String,
    pub username: String,
    pub password: String,
    pub autologin: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Plan {
    pub mode: Mode,
    /// Whole disk (erase mode) or the disk holding `root` (other modes).
    pub disk: String,
    /// Must equal [`confirm_token`] of the disk as the backend sees it now,
    /// proving the user confirmed exactly this device.
    pub confirm: String,
    pub esp: Option<String>,
    pub format_esp: bool,
    /// Partition that becomes the Btrfs (or LUKS) system partition.
    pub root: Option<String>,
    /// Separate unencrypted /boot, required with encryption in manual mode.
    pub boot: Option<String>,
    /// LUKS passphrase: new container (erase/manual) or unlock (reinstall).
    pub passphrase: Option<String>,
    pub encrypt: bool,
    /// None: the account is created by MAN Setup (OOBE) on first start.
    pub user: Option<User>,
    pub root_password: Option<String>,
    pub hostname: String,
    pub timezone: String,
    pub keyboard: String,
    /// System language (locale) of the installed system; MAN Setup offers
    /// it first. None: the live session's language.
    #[serde(default)]
    pub language: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct SnapshotInfo {
    pub id: String,
    /// RFC 3339, UTC.
    pub created: String,
    pub description: String,
    /// install, manual, daily, pre-update, pre-rollback, pre-reinstall
    pub kind: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SnapshotConfig {
    pub daily: bool,
    pub keep_daily: u32,
    pub pre_update: bool,
    pub keep_pre_update: u32,
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self { daily: true, keep_daily: 7, pre_update: true, keep_pre_update: 5 }
    }
}

impl SnapshotConfig {
    pub fn parse(text: &str) -> Self {
        let mut c = Self::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            let (k, v) = (k.trim(), v.trim());
            match k {
                "DAILY" => c.daily = v == "yes",
                "KEEP_DAILY" => c.keep_daily = v.parse().unwrap_or(c.keep_daily),
                "PRE_UPDATE" => c.pre_update = v == "yes",
                "KEEP_PRE_UPDATE" => c.keep_pre_update = v.parse().unwrap_or(c.keep_pre_update),
                _ => {}
            }
        }
        c
    }

    pub fn render(&self) -> String {
        let yn = |b: bool| if b { "yes" } else { "no" };
        format!(
            "# MAN system snapshots. Managed by the Snapshots app; edits are kept.\n\
             DAILY={}\nKEEP_DAILY={}\nPRE_UPDATE={}\nKEEP_PRE_UPDATE={}\n",
            yn(self.daily), self.keep_daily, yn(self.pre_update), self.keep_pre_update
        )
    }
}

/// Name of the locked system account that runs MAN Setup on first start.
pub const OOBE_USER: &str = "man-oobe";
/// Exists while MAN Setup has not been completed.
pub const OOBE_PENDING: &str = "/var/lib/man/oobe-pending";
/// Exists while the disk is waiting to be encrypted at the next start.
pub const ENCRYPT_PENDING: &str = "/etc/man/encrypt-pending";

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct Appearance {
    /// "default" (light) or "prefer-dark", as org.gnome.desktop.interface.
    pub color_scheme: String,
    /// GNOME accent name: blue, teal, green, yellow, orange, red, pink, purple, slate.
    pub accent: String,
    pub wallpaper: String,
    pub wallpaper_dark: String,
}

pub const ACCENTS: &[(&str, &str, &str)] = &[
    ("blue", "Blue", "#3584e4"),
    ("teal", "Teal", "#2190a4"),
    ("green", "Green", "#3a944a"),
    ("yellow", "Yellow", "#c88800"),
    ("orange", "Orange", "#ed5b00"),
    ("red", "Red", "#e62d42"),
    ("pink", "Pink", "#d56199"),
    ("purple", "Purple", "#9141ac"),
    ("slate", "Slate", "#6f8396"),
];

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct OobeConfig {
    pub user: User,
    pub root_password: Option<String>,
    pub hostname: String,
    pub appearance: Appearance,
    pub location: bool,
    pub automatic_timezone: bool,
    /// Encrypt the system disk at the next start, with this passphrase.
    pub encrypt_passphrase: Option<String>,
    /// System and account language, e.g. "cs_CZ.UTF-8".
    #[serde(default)]
    pub language: String,
    /// Country (ISO 3166 alpha-2): formats and, if none was set, time zone.
    #[serde(default)]
    pub country: String,
}

/// Trusted public keys for signed `.upd` update files (`*.pub`, base64 ed25519).
pub const UPDATE_KEYS_DIR: &str = "/usr/share/man/update-keys";
/// Version of the installed MAN system (written by the installer and updates).
pub const VERSION_FILE: &str = "/usr/lib/man/version";
/// Details of the last installed update, read by the login pop-up.
pub const LAST_UPDATE: &str = "/var/lib/man/last-update.json";
pub const UPD_FORMAT: u32 = 1;

/// `manifest.json` inside a `.upd` file. Signed as a whole (`manifest.sig`).
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct UpdateManifest {
    pub format: u32,
    pub name: String,
    pub version: String,
    /// Build date, YYYY-MM-DD.
    pub date: String,
    /// sha256 of `system.tar.zst`, lowercase hex.
    pub payload_sha256: String,
    pub payload_size: u64,
    /// Space the unpacked system needs.
    pub installed_size: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct UpdateTarget {
    pub disk: String,
    pub model: String,
    pub device: String,
    pub encrypted: bool,
    pub running: bool,
    pub free: u64,
    /// Why this installation cannot be updated, if it cannot.
    pub problem: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct UpdateInfo {
    pub manifest: UpdateManifest,
    pub license: String,
    pub notes: String,
    pub signed_by: String,
    pub current_version: String,
    pub targets: Vec<UpdateTarget>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct LastUpdate {
    pub version: String,
    pub previous: String,
    pub date: String,
}

/// Compare dotted versions numerically ("10.10" > "10.9").
pub fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let parts = |s: &str| s.split(|c: char| !c.is_ascii_alphanumeric()).map(|p| p.parse::<u64>().map_err(|_| p.to_string())).collect::<Vec<_>>();
    let (pa, pb) = (parts(a), parts(b));
    for i in 0..pa.len().max(pb.len()) {
        let o = match (pa.get(i), pb.get(i)) {
            (Some(Ok(x)), Some(Ok(y))) => x.cmp(y),
            (Some(Err(x)), Some(Err(y))) => x.cmp(y),
            (Some(Ok(_)), Some(Err(_))) => std::cmp::Ordering::Greater,
            (Some(Err(_)), Some(Ok(_))) => std::cmp::Ordering::Less,
            (Some(_), None) => std::cmp::Ordering::Greater,
            (None, Some(_)) => std::cmp::Ordering::Less,
            (None, None) => std::cmp::Ordering::Equal,
        };
        if o != std::cmp::Ordering::Equal {
            return o;
        }
    }
    std::cmp::Ordering::Equal
}

pub fn valid_version(v: &str) -> bool {
    !v.is_empty() && v.len() <= 32 && v.chars().all(|c| c.is_ascii_alphanumeric() || ".-+~".contains(c))
}

/// Requests understood by `man-installer-backend request` on stdin.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    Probe,
    /// Unlock an encrypted install read-only and report what it contains.
    Inspect { device: String, passphrase: String },
    Install { plan: Plan },
    SnapshotCreate { description: String },
    SnapshotDelete { id: String },
    Rollback { id: String },
    SetSnapshotConfig { config: SnapshotConfig },
    /// Finish MAN Setup: create the account, apply settings, schedule encryption.
    OobeApply { config: OobeConfig },
    /// Live session: switch to this language (takes effect at the next login).
    SetLiveLanguage { locale: String },
    /// Verify a `.upd` file and report what it contains.
    UpdateInspect { path: String },
    /// Prepare the update in @update; it is applied at the next restart.
    UpdateStage { path: String, device: String, snapshot: bool },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    /// A new installation phase started.
    Step { index: u32, total: u32, title: String },
    Progress { fraction: f64, detail: String },
    Log { line: String },
    Probe { probe: Probe },
    Inspect { install: ExistingInstall },
    Update { info: UpdateInfo },
    Done { message: String },
    Error { message: String, detail: String },
}

pub fn confirm_token(disk: &Disk) -> String {
    format!("{}|{}|{}", disk.path, disk.size, disk.serial)
}

pub fn valid_username(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some('a'..='z' | '_'))
        && s.len() <= 32
        && chars.all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_' | '-'))
        && !RESERVED_USERS.contains(&s)
}

/// System accounts the image already has, plus the live accounts.
pub const RESERVED_USERS: &[&str] = &[
    "root", "daemon", "bin", "sys", "sync", "games", "man", "lp", "mail", "news", "uucp",
    "proxy", "www-data", "backup", "list", "irc", "nobody", "messagebus", "polkitd",
    "colord", "geoclue", "pipewire", "systemd-network", "gdm", "gdm-greeter", "maninstaller",
    "man-oobe", "debian-gdm",
];

pub fn valid_hostname(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 63
        && !s.starts_with('-')
        && !s.ends_with('-')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Snapshot descriptions travel through GRUB menus and the kernel command
/// line tooling, so they are restricted to a safe character set.
pub fn sanitize_description(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_alphanumeric() || " .,:()+_-/".contains(*c))
        .take(80)
        .collect();
    let t = cleaned.trim();
    if t.is_empty() { "Snapshot".into() } else { t.into() }
}

pub fn valid_snapshot_id(s: &str) -> bool {
    s.len() == 15 && s.chars().enumerate().all(|(i, c)| if i == 8 { c == '-' } else { c.is_ascii_digit() })
}

pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut u = 0;
    while v >= 1000.0 && u < UNITS.len() - 1 {
        v /= 1000.0;
        u += 1;
    }
    if u == 0 { format!("{bytes} B") } else if v >= 100.0 { format!("{v:.0} {}", UNITS[u]) } else { format!("{v:.1} {}", UNITS[u]) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usernames() {
        assert!(valid_username("anna"));
        assert!(valid_username("a_b-9"));
        assert!(!valid_username("Anna"));
        assert!(!valid_username("9lives"));
        assert!(!valid_username("root"));
        assert!(!valid_username(""));
    }

    #[test]
    fn hostnames() {
        assert!(valid_hostname("man-pc"));
        assert!(!valid_hostname("-x"));
        assert!(!valid_hostname("a b"));
    }

    #[test]
    fn snapshot_ids() {
        assert!(valid_snapshot_id("20261018-153012"));
        assert!(!valid_snapshot_id("2026101-1530120"));
        assert!(!valid_snapshot_id("20261018-15301x"));
    }

    #[test]
    fn descriptions() {
        assert_eq!(sanitize_description("  'rm -rf' \"x\" "), "rm -rf x");
        assert_eq!(sanitize_description("\"\""), "Snapshot");
    }

    #[test]
    fn config_roundtrip() {
        let c = SnapshotConfig { daily: false, keep_daily: 3, pre_update: true, keep_pre_update: 9 };
        assert_eq!(SnapshotConfig::parse(&c.render()), c);
    }

    #[test]
    fn versions() {
        use std::cmp::Ordering::*;
        assert_eq!(version_cmp("10.10", "10.9"), Greater);
        assert_eq!(version_cmp("10.1", "10.1"), Equal);
        assert_eq!(version_cmp("10.0", "10.0.1"), Less);
        assert!(valid_version("10.1-rc1"));
        assert!(!valid_version("1 0"));
    }

    #[test]
    fn request_json() {
        let r: Request = serde_json::from_str(r#"{"op":"rollback","id":"20261018-153012"}"#).unwrap();
        assert!(matches!(r, Request::Rollback { .. }));
    }
}
