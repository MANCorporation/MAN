//! MAN Recovery: a small MAN kept in the `@recovery` subvolume next to
//! `@system`, started from the boot menu ("MAN Recovery"). It restores a
//! snapshot, reinstalls MAN from the newest ISO (keeping files and accounts)
//! or opens a Terminal, after an administrator's password.
//!
//! Made by the installer (a snapshot of the freshly copied system, before
//! the installer is removed from it; from the downloaded ISO when Recovery
//! itself reinstalls) and by updates (a snapshot of the new system), so it
//! always has the installer and shares its disk space with the system. Its kernel and initramfs are the
//! installed system's, copied to /boot/man-recovery (separate /boot) or
//! @recovery/boot/man-recovery; the initramfs puts a RAM overlay over the
//! read-only @recovery (man-recovery in local-bottom).
use crate::util::*;
use crate::{install, probe, snapshots, update};
use anyhow::{anyhow, bail, Context, Result};
use man_common::*;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

pub const SUBVOL: &str = "@recovery";
/// In the installed system: the recovery's MAN version and kernel (read by
/// the boot menu and the initramfs update hook).
pub const SYSTEM_MARK: &str = "var/lib/man/recovery";
const BOOT_DIR: &str = "boot/man-recovery";
const USER: &str = "man-recovery";
const TOP: &str = "/run/man-recovery/top";
const DOWNLOAD: &str = "@man-recovery-download";
const API: &str = "https://api.github.com/repos/MANCorporation/MAN";

/// Left out of the recovery system: everyday apps and data it never uses.
const STRIP: &[&str] = &[
    "/boot/*", "/usr/lib/*/wine", "/usr/lib/wine", "/usr/share/wine", "/opt/*",
    "/var/lib/flatpak/*", "/var/lib/clamav/*", "/var/lib/apt/lists/*", "/var/cache/*",
    "/var/lib/swcatalog/*", "/usr/share/swcatalog/*", "/var/lib/man-login/*", "/var/lib/man/kept-debs/*",
    "/usr/share/help/*", "/usr/share/doc/*", "/usr/share/man/*", "/usr/share/info/*", "/usr/share/gnome/help/*",
    "/usr/lib/*/libwebkit*", "/usr/lib/*/libjavascriptcoregtk*", "/usr/lib/*/webkit2gtk-*", "/usr/lib/*/webkitgtk-*",
    "/usr/share/man-sample-music", "/etc/skel/Music/*",
];
/// Translations Recovery shows (its own, MAN Installer's, GTK's and Console's);
/// the others are left out.
const KEEP_LOCALE: &[&str] = &["man-installer.mo", "gtk40.mo", "gtk40-properties.mo", "libadwaita.mo", "glib20.mo", "kgx.mo", "gnome-console.mo"];
/// Machine settings taken over from the installed system.
const FROM_SYSTEM: &[&str] = &[
    "etc/hostname", "etc/hosts", "etc/localtime", "etc/timezone", "etc/default/keyboard", "etc/default/locale",
    "etc/locale.conf", "etc/vconsole.conf", "etc/dconf/db/local.d/00-man-keyboard", "etc/NetworkManager/system-connections",
];
/// Units of the installed system that have nothing to do in Recovery.
const MASK: &[(&str, &str)] = &[
    ("system", "man-snapshot-daily.timer"), ("system", "man-boot-cleanup.timer"), ("system", "man-boot-cleanup.service"),
    ("system", "man-update-finish.service"), ("system", "man-encrypt.service"), ("system", "man-duhach-cleanup.service"),
    ("system", "man-oobe-cleanup.service"), ("system", "man-kept-debs.service"), ("system", "man-apple-next.service"),
    ("user", "man-update-check.timer"), ("user", "man-download-scan.service"),
];

// ─────────────────────────── Making it ───────────────────────────

/// Start a new recovery: a writable snapshot of `from`, a whole MAN that
/// still has the installer (the system just copied by the installer, or a
/// new system from an update). It shares its disk space with that system.
pub fn begin(top: &Path, from: &Path) -> Result<()> {
    if !from.join("usr/bin/man-recovery").exists() {
        bail!("This MAN has no MAN Recovery program.");
    }
    let new = top.join(format!("{SUBVOL}.new"));
    if new.exists() {
        snapshots::delete_subvolume_tree(top, &format!("{SUBVOL}.new"))?;
    }
    run("btrfs", &["subvolume", "snapshot", from.to_str().unwrap(), new.to_str().unwrap()])?;
    Ok(())
}

/// Finish the recovery started by [`begin`]: strip it, set it up from the
/// installed system `sys` and give it the kernel `sys` boots. `boot` is
/// where the installed system's /boot is mounted, when it is a separate
/// partition.
pub fn finish(top: &Path, sys: &Path, boot: Option<&Path>, progress: &dyn Fn(f64, &str)) -> Result<()> {
    let name = format!("{SUBVOL}.new");
    let new = top.join(&name);
    if !new.exists() {
        bail!("MAN Recovery was not started.");
    }
    let result = (|| -> Result<String> {
        progress(0.1, "Leaving out what Recovery does not need");
        strip(&new)?;
        progress(0.5, "Setting up the recovery system");
        configure(&new, sys)?;
        progress(0.8, "Adding its start-up files");
        let kernel = install_boot(&new, sys, boot)?;
        // Put it in place. The one Recovery itself runs from cannot be
        // deleted now: it is renamed and removed at the next normal start.
        let cur = top.join(SUBVOL);
        if cur.exists() && snapshots::delete_subvolume_tree(top, SUBVOL).is_err() {
            std::fs::rename(&cur, top.join(format!("{SUBVOL}.old-{}", snapshot_id(now()))))?;
        }
        std::fs::rename(&new, &cur)?;
        Ok(kernel)
    })();
    match result {
        Ok(kernel) => {
            let version = read_to_string(sys.join(VERSION_FILE.trim_start_matches('/'))).unwrap_or_default();
            write_file(sys.join(SYSTEM_MARK), &format!("VERSION={}\nKERNEL={kernel}\n", version.trim()), 0o644)?;
            progress(1.0, "MAN Recovery is ready");
            Ok(())
        }
        Err(e) => {
            if new.exists() {
                let _ = snapshots::delete_subvolume_tree(top, &name);
            }
            Err(e)
        }
    }
}

/// Remove from `r` what Recovery never uses.
fn strip(r: &Path) -> Result<()> {
    let rs = r.to_str().unwrap();
    // Constant patterns, expanded by the shell inside the recovery root.
    let patterns: Vec<String> = STRIP.iter().chain(install::EXCLUDES).map(|p| format!(".{p}")).collect();
    run("sh", &["-c", &format!("cd \"$1\" && rm -rf -- {}", patterns.join(" ")), "sh", rs])?;
    let mut find = vec![format!("{rs}/usr/share/locale"), "-path".into(), "*/LC_MESSAGES/*".into(), "-type".into(), "f".into()];
    for f in KEEP_LOCALE {
        find.extend(["!".into(), "-name".into(), f.to_string()]);
    }
    find.push("-delete".into());
    let args: Vec<&str> = find.iter().map(String::as_str).collect();
    let _ = run("find", &args);
    Ok(())
}

fn copy_from(sys: &Path, r: &Path, rel: &str) -> Result<()> {
    let src = sys.join(rel);
    if std::fs::symlink_metadata(&src).is_err() {
        return Ok(());
    }
    let dst = r.join(rel);
    if let Ok(m) = std::fs::symlink_metadata(&dst) {
        if m.is_dir() && !m.file_type().is_symlink() {
            std::fs::remove_dir_all(&dst)?;
        } else {
            std::fs::remove_file(&dst)?;
        }
    }
    std::fs::create_dir_all(dst.parent().unwrap())?;
    run("cp", &["-a", src.to_str().unwrap(), dst.to_str().unwrap()])?;
    Ok(())
}

fn configure(r: &Path, sys: &Path) -> Result<()> {
    let rs = r.to_str().unwrap();
    let version = read_to_string(r.join(VERSION_FILE.trim_start_matches('/'))).unwrap_or_default();
    write_file(r.join(RECOVERY_MARK.trim_start_matches('/')), &format!("{}\n", version.trim()), 0o644)?;
    write_file(r.join("etc/fstab"), "# MAN Recovery: its system is read-only; changes are kept in memory until it restarts.\n", 0o644)?;
    write_file(r.join("etc/machine-id"), "", 0o444)?;
    for rel in FROM_SYSTEM {
        copy_from(sys, r, rel)?;
    }
    let _ = chroot(r, &["dconf", "update"]);

    // The live session's account and helpers are not part of Recovery.
    let _ = chroot(r, &["userdel", "-f", "MANInstaller"]);
    let _ = std::fs::remove_dir_all(r.join("home/MANInstaller"));
    for f in ["etc/sudoers.d/90-maninstaller", "etc/polkit-1/rules.d/49-man-live.rules", OOBE_PENDING.trim_start_matches('/')] {
        let _ = std::fs::remove_file(r.join(f));
    }

    // Recovery runs as its own locked account, in the MAN Recovery session.
    if query("chroot", &[rs, "id", "-u", USER]).is_none() {
        chroot(r, &["useradd", "-r", "-m", "-d", "/var/lib/man-recovery", "-s", "/bin/bash", "-c", "MAN Recovery", USER])?;
    }
    chroot(r, &["usermod", "-p", "!", USER])?;
    let groups: Vec<&str> = ["audio", "video", "input", "render", "netdev"].into_iter().filter(|g| query("chroot", &[rs, "getent", "group", g]).is_some()).collect();
    chroot(r, &["usermod", "-a", "-G", &groups.join(","), USER])?;
    write_file(r.join("var/lib/AccountsService/users").join(USER), "[User]\nSession=man-recovery\nSystemAccount=true\n", 0o600)?;
    let gdm = r.join("etc/gdm3/daemon.conf");
    let text = read_to_string(&gdm).unwrap_or_default();
    write_file(&gdm, &install::gdm_config(&text, Some(USER), Some("man-recovery")), 0o644)?;
    // MAN Recovery asks for an administrator's password of the installed
    // system before it does anything; its session itself may do everything.
    write_file(r.join("etc/sudoers.d/90-man-recovery"), &format!("{USER} ALL=(ALL:ALL) NOPASSWD: ALL\n"), 0o440)?;
    write_file(
        r.join("etc/polkit-1/rules.d/49-man-recovery.rules"),
        &format!("// MAN Recovery: its session may manage the computer (it checks an administrator's password itself).\npolkit.addRule(function (action, subject) {{\n    if (subject.user == \"{USER}\" && subject.local && subject.active)\n        return polkit.Result.YES;\n    return polkit.Result.NOT_HANDLED;\n}});\n"),
        0o644,
    )?;
    for (kind, unit) in MASK {
        let dir = r.join(format!("etc/systemd/{kind}"));
        std::fs::create_dir_all(&dir)?;
        let link = dir.join(unit);
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink("/dev/null", &link)?;
    }
    Ok(())
}

/// Kernel + initramfs of the installed system for the recovery: the newest
/// kernel whose modules the recovery has. Returns the kernel version.
fn install_boot(r: &Path, sys: &Path, boot: Option<&Path>) -> Result<String> {
    let mut versions: Vec<String> = std::fs::read_dir(r.join("usr/lib/modules"))?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|v| sys.join(format!("boot/vmlinuz-{v}")).exists() && sys.join(format!("boot/initrd.img-{v}")).exists())
        .collect();
    versions.sort_by(|a, b| version_sort_key(a).cmp(&version_sort_key(b)));
    let v = versions.pop().ok_or_else(|| anyhow!("The installed system has no kernel for MAN Recovery."))?;
    let mut dirs = vec![r.join(BOOT_DIR)];
    if let Some(b) = boot {
        dirs.push(b.join("man-recovery"));
    }
    for d in dirs {
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d)?;
        std::fs::copy(sys.join(format!("boot/vmlinuz-{v}")), d.join("vmlinuz"))?;
        std::fs::copy(sys.join(format!("boot/initrd.img-{v}")), d.join("initrd.img"))?;
        write_file(d.join("version"), &format!("{v}\n"), 0o644)?;
    }
    Ok(v)
}

fn version_sort_key(v: &str) -> Vec<u64> {
    v.split(|c: char| !c.is_ascii_digit()).filter_map(|n| n.parse().ok()).collect()
}

// ─────────────────────────── Inside Recovery ───────────────────────────

pub fn active() -> bool {
    Path::new(RECOVERY_MARK).exists()
}

/// The Btrfs device Recovery started from (root=UUID=… on the command line).
fn device() -> Result<String> {
    let cmdline = read_to_string("/proc/cmdline").unwrap_or_default();
    let uuid = cmdline.split_whitespace().find_map(|a| a.strip_prefix("root=UUID=")).ok_or_else(|| anyhow!("MAN Recovery was not started from the boot menu."))?;
    query("blkid", &["-U", uuid]).ok_or_else(|| anyhow!("The disk of this MAN was not found."))
}

fn mount_top(m: &mut Mounts) -> Result<PathBuf> {
    if !active() {
        bail!("This works only in MAN Recovery.");
    }
    let top = PathBuf::from(TOP);
    m.mount(&device()?, &top, &["-o", "subvolid=5"])?;
    Ok(top)
}

fn passwd_entries(sys: &Path) -> Vec<Vec<String>> {
    read_to_string(sys.join("etc/passwd")).unwrap_or_default().lines().map(|l| l.split(':').map(String::from).collect()).collect()
}

fn admins(sys: &Path) -> Vec<RecoveryUser> {
    let group = read_to_string(sys.join("etc/group")).unwrap_or_default();
    let members: Vec<String> = group
        .lines()
        .filter(|l| l.starts_with("sudo:") || l.starts_with("wheel:"))
        .flat_map(|l| l.rsplit(':').next().unwrap_or("").split(',').map(|s| s.trim().to_string()).collect::<Vec<_>>())
        .filter(|s| !s.is_empty())
        .collect();
    passwd_entries(sys)
        .into_iter()
        .filter(|f| f.len() >= 7 && members.contains(&f[0]) && f[2].parse::<u32>().is_ok_and(|u| (1000..60000).contains(&u)))
        .map(|f| RecoveryUser { real_name: f[4].split(',').next().filter(|s| !s.is_empty()).unwrap_or(&f[0]).to_string(), name: f[0].clone() })
        .collect()
}

pub fn info() -> Result<RecoveryInfo> {
    let mut m = Mounts::default();
    let top = mount_top(&mut m)?;
    let sys = top.join("@system");
    let mut snapshots: Vec<SnapshotInfo> = std::fs::read_dir(top.join("@snapshots"))
        .map(|r| {
            r.filter_map(|e| e.ok())
                .filter(|e| e.path().join("snapshot").is_dir())
                .filter_map(|e| read_to_string(e.path().join("info.json")))
                .filter_map(|t| serde_json::from_str(&t).ok())
                .collect()
        })
        .unwrap_or_default();
    snapshots.sort_by(|a: &SnapshotInfo, b| b.id.cmp(&a.id));
    Ok(RecoveryInfo {
        version: read_to_string(sys.join(VERSION_FILE.trim_start_matches('/'))).unwrap_or_default().trim().to_string(),
        admins: admins(&sys),
        snapshots,
        encrypted: device()?.starts_with("/dev/mapper/") || device()?.starts_with("/dev/dm-"),
    })
}

/// Check `password` against the installed system's own account database.
fn check_password(sys: &Path, user: &str, password: &str) -> Result<()> {
    let wrong = || anyhow!("The password is not correct.");
    if !admins(sys).iter().any(|a| a.name == user) {
        return Err(wrong());
    }
    let shadow = std::fs::read_to_string(sys.join("etc/shadow")).context("cannot read the accounts of the installed system")?;
    let hash = shadow
        .lines()
        .find_map(|l| l.strip_prefix(&format!("{user}:")))
        .and_then(|rest| rest.split(':').next())
        .unwrap_or("")
        .to_string();
    if hash.is_empty() || hash.starts_with('!') || hash.starts_with('*') {
        return Err(wrong());
    }
    // Perl's crypt() is the system's libcrypt (yescrypt, SHA-512, …). The
    // password goes through stdin, never the command line.
    let mut child = Command::new("perl")
        .args(["-e", "my $p = <STDIN>; chomp $p; print crypt($p, $ARGV[0]) eq $ARGV[0] ? 'ok' : 'no';", &hash])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    child.stdin.take().unwrap().write_all(format!("{password}\n").as_bytes())?;
    let out = child.wait_with_output()?;
    if String::from_utf8_lossy(&out.stdout) == "ok" {
        Ok(())
    } else {
        std::thread::sleep(Duration::from_secs(2));
        Err(wrong())
    }
}

pub fn unlock(user: &str, password: &str) -> Result<String> {
    let mut m = Mounts::default();
    let top = mount_top(&mut m)?;
    check_password(&top.join("@system"), user, password)?;
    Ok("Unlocked".into())
}

/// Make snapshot `id` the system again, like the boot menu's restore: the
/// current system is kept as a snapshot first.
pub fn restore(user: &str, password: &str, id: &str) -> Result<String> {
    if !valid_snapshot_id(id) {
        bail!("Invalid snapshot id.");
    }
    let mut m = Mounts::default();
    let top = mount_top(&mut m)?;
    check_password(&top.join("@system"), user, password)?;
    let snap_dir = top.join("@snapshots").join(id);
    let target: SnapshotInfo = read_to_string(snap_dir.join("info.json")).and_then(|t| serde_json::from_str(&t).ok()).ok_or_else(|| anyhow!("The snapshot {id} does not exist."))?;
    let now_id = snapshot_id(now());
    if top.join("@system").is_dir() {
        let dir = top.join("@snapshots").join(&now_id);
        std::fs::create_dir_all(&dir)?;
        run("btrfs", &["subvolume", "snapshot", "-r", top.join("@system").to_str().unwrap(), dir.join("snapshot").to_str().unwrap()])?;
        snapshots::write_info(&dir, &SnapshotInfo { id: now_id.clone(), created: rfc3339(now()), description: format!("Before restoring {}", target.description), kind: "pre-rollback".into() })?;
        std::fs::rename(top.join("@system"), top.join(format!("@system.old-{now_id}")))?;
    }
    if let Err(e) = run("btrfs", &["subvolume", "snapshot", snap_dir.join("snapshot").to_str().unwrap(), top.join("@system").to_str().unwrap()]) {
        let _ = std::fs::rename(top.join(format!("@system.old-{now_id}")), top.join("@system"));
        return Err(e);
    }
    write_file(top.join("@system/var/lib/man/rollback-done"), &format!("{id}\n"), 0o644)?;
    // GRUB may read its menu from @system: never leave it without one.
    let cfg = top.join("@system/boot/grub/grub.cfg");
    let old_cfg = top.join(format!("@system.old-{now_id}/boot/grub/grub.cfg"));
    if !cfg.exists() && old_cfg.exists() {
        std::fs::create_dir_all(cfg.parent().unwrap())?;
        std::fs::copy(&old_cfg, &cfg)?;
    }
    Ok(format!("Your computer was restored to “{}”. Restart to use it.", target.description))
}

fn step(index: u32, total: u32, title: &str) {
    emit(&Event::Step { index, total, title: title.into() });
}

fn arch() -> &'static str {
    if probe::is_apple_silicon() { "apple" } else { SYSTEM_ARCH }
}

fn curl_json(url: &str) -> Result<serde_json::Value> {
    let out = Command::new("curl")
        .args(["-fsSL", "--max-time", "60", "-A", "MAN-Recovery", "-H", "Accept: application/vnd.github+json", url])
        .stdin(Stdio::null())
        .output()
        .context("curl")?;
    if !out.status.success() {
        bail!("MAN's download page could not be reached. Check the internet connection. ({})", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(serde_json::from_slice(&out.stdout)?)
}

/// The newest MAN ISO for this computer among the published releases
/// (TEST versions included: MAN 10 comes as TEST versions).
pub fn find_release() -> Result<RecoveryRelease> {
    let api = std::env::var("MAN_UPDATE_API").unwrap_or_else(|_| API.into());
    let v = curl_json(&format!("{api}/releases?per_page=30"))?;
    let mut best: Option<RecoveryRelease> = None;
    for rel in v.as_array().cloned().unwrap_or_default() {
        if rel["draft"].as_bool() == Some(true) {
            continue;
        }
        let assets = rel["assets"].as_array().cloned().unwrap_or_default();
        for a in &assets {
            let name = a["name"].as_str().unwrap_or("");
            let Some(version) = iso_file_version(name, arch()) else { continue };
            let sha_name = format!("{name}.sha256");
            let Some(sha) = assets.iter().find(|s| s["name"].as_str() == Some(&sha_name)).and_then(|s| s["browser_download_url"].as_str()) else { continue };
            let r = RecoveryRelease { version, size: a["size"].as_u64().unwrap_or(0), url: a["browser_download_url"].as_str().unwrap_or("").into(), sha256_url: sha.into() };
            if !r.url.is_empty() && best.as_ref().is_none_or(|b| man_version_cmp(&r.version, "", &b.version, "").is_gt()) {
                best = Some(r);
            }
        }
    }
    best.ok_or_else(|| anyhow!("No MAN for this computer was found on MAN's download page."))
}

fn download(r: &RecoveryRelease, dest: &Path) -> Result<()> {
    let mut child = Command::new("curl")
        .args(["-fsSL", "--retry", "3", "-A", "MAN-Recovery", "-o", dest.to_str().unwrap(), &r.url])
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("curl")?;
    loop {
        if let Some(status) = child.try_wait()? {
            if !status.success() {
                let mut err = String::new();
                let _ = std::io::Read::read_to_string(&mut child.stderr.take().unwrap(), &mut err);
                bail!("The download stopped: {}", err.trim());
            }
            break;
        }
        let got = std::fs::metadata(dest).map(|m| m.len()).unwrap_or(0);
        if r.size > 0 {
            emit(&Event::Progress { fraction: (got as f64 / r.size as f64).min(1.0), detail: format!("{} of {}", human_size(got), human_size(r.size)) });
        }
        std::thread::sleep(Duration::from_millis(700));
    }
    let out = Command::new("curl").args(["-fsSL", "--retry", "3", &r.sha256_url]).output()?;
    let expected = String::from_utf8_lossy(&out.stdout).split_whitespace().next().unwrap_or("").to_lowercase();
    let actual = run("sha256sum", &[dest.to_str().unwrap()])?.split_whitespace().next().unwrap_or("").to_string();
    if expected.len() != 64 || expected != actual {
        bail!("The downloaded MAN is damaged (its checksum does not match). Try again.");
    }
    Ok(())
}

/// Download the newest MAN and install it again on this computer, keeping
/// files and accounts (the current system becomes a snapshot).
pub fn reinstall(user: &str, password: &str, version: &str) -> Result<String> {
    let release = find_release()?;
    if release.version != version {
        bail!("MAN {version} is not offered any more; MAN {} is. Start again.", release.version);
    }
    let mut m = Mounts::default();
    let top = mount_top(&mut m)?;
    check_password(&top.join("@system"), user, password)?;
    let total = 4u32;
    step(1, total, "Downloading MAN");
    let dl = top.join(DOWNLOAD);
    if dl.exists() {
        snapshots::delete_subvolume_tree(&top, DOWNLOAD)?;
    }
    run("btrfs", &["subvolume", "create", dl.to_str().unwrap()])?;
    let iso = dl.join("man.iso");
    let result = (|| -> Result<String> {
        download(&release, &iso)?;
        step(2, total, "Opening the downloaded MAN");
        let mut src = Mounts::default();
        let iso_dir = PathBuf::from("/run/man-recovery/iso");
        src.mount(iso.to_str().unwrap(), &iso_dir, &["-o", "loop,ro"])?;
        let source = PathBuf::from("/run/man-recovery/source");
        src.mount(iso_dir.join("live/filesystem.squashfs").to_str().unwrap(), &source, &["-t", "squashfs", "-o", "loop,ro"])?;
        if let Some(size) = read_to_string(iso_dir.join("live/filesystem.size")).and_then(|s| s.trim().parse().ok()) {
            install::SOURCE_BYTES.store(size, std::sync::atomic::Ordering::Relaxed);
        }
        let plan = reinstall_plan(&top.join("@system"), &source)?;
        drop(m);
        step(3, total, "Installing MAN");
        let msg = install::install(plan)?;
        src.release();
        Ok(msg)
    })();
    let mut again = Mounts::default();
    if let Ok(top) = mount_top(&mut again) {
        let _ = snapshots::delete_subvolume_tree(&top, DOWNLOAD);
    }
    let _ = result?;
    Ok(format!("MAN {} was installed again. Your files and accounts were kept. Restart to use it.", release.version))
}

/// The plan for reinstalling the system Recovery belongs to.
fn reinstall_plan(sys: &Path, source: &Path) -> Result<Plan> {
    let dev = device()?;
    // The partition: the Btrfs itself, or the LUKS container under it.
    let tree = query("lsblk", &["-snlo", "PATH,TYPE", &dev]).unwrap_or_default();
    let root = tree.lines().filter_map(|l| {
        let mut it = l.split_whitespace();
        Some((it.next()?.to_string(), it.next()?.to_string()))
    }).find(|(_, t)| t == "part").map(|(p, _)| p).ok_or_else(|| anyhow!("The partition of this MAN was not found."))?;
    let disks = probe::disks()?;
    let disk = disks.iter().find(|d| d.partitions.iter().any(|p| p.path == root)).ok_or_else(|| anyhow!("The disk of this MAN was not found."))?;
    let get = |rel: &str| read_to_string(sys.join(rel)).unwrap_or_default();
    let keyboard = {
        let k = get("etc/default/keyboard");
        let val = |key: &str| k.lines().find_map(|l| l.strip_prefix(&format!("{key}="))).unwrap_or("").trim_matches('"').to_string();
        let (layout, variant) = (val("XKBLAYOUT"), val("XKBVARIANT"));
        let layout = layout.split(',').next().unwrap_or("us").to_string();
        if variant.is_empty() { if layout.is_empty() { "us".into() } else { layout } } else { format!("{layout}+{}", variant.split(',').next().unwrap_or("")) }
    };
    let timezone = get("etc/timezone").trim().to_string();
    let hostname = get("etc/hostname").trim().to_string();
    let hostname = if valid_hostname(&hostname) { hostname } else { "man".into() };
    let language = get("etc/default/locale").lines().find_map(|l| l.strip_prefix("LANG=")).map(|s| s.trim_matches('"').to_string());
    Ok(Plan {
        mode: Mode::Reinstall,
        disk: disk.path.clone(),
        confirm: confirm_token(disk),
        esp: None,
        format_esp: false,
        root: Some(root),
        boot: None,
        passphrase: None,
        encrypt: false,
        user: None,
        root_password: None,
        hostname,
        timezone: if timezone.is_empty() { "UTC".into() } else { timezone },
        keyboard,
        language: language.filter(|l| valid_locale(l)),
        source: Some(source.to_string_lossy().into_owned()),
        keep_accounts: true,
    })
}

/// The kept accounts and machine settings for a reinstall (`keep_accounts`).
pub fn keep_machine(old: &Path, new: &Path) -> Result<()> {
    update::migrate_from(old, new)
}
