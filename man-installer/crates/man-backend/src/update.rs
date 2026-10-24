//! System updates from signed `.upd` files.
//!
//! Staging (running system): the new system is unpacked into the `@update`
//! subvolume, accounts and machine settings are carried over, its boot files
//! are prepared, and a one-time boot entry is set. At the restart, the
//! initramfs (`man-update` script) swaps `@update` in as `@system` and
//! restarts once more. The new system finishes in `man-update-finish.service`
//! and the user sees a notice after logging in. `/home` is never touched.
use crate::probe;
use crate::snapshots;
use crate::updfile::{self, PAYLOAD};
use crate::util::*;
use anyhow::{anyhow, bail, Context, Result};
use man_common::*;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const TOP: &str = "/run/man-update/top";
const NEWROOT: &str = "/run/man-update/new";
/// In the running (old) system: the staged update waiting for the restart.
const STAGED: &str = "/var/lib/man/update-staged";
/// Kernel and initramfs used for the one boot that applies the update.
const UPDATE_BOOT: &str = "/boot/man-update";
/// In the new system: set by the initramfs after the swap.
const APPLIED: &str = "/var/lib/man/update-applied";
const INFO: &str = "/var/lib/man/update-info.json";
const HEADROOM: u64 = 2 << 30;

const STEPS: [&str; 6] = [
    "Checking the update",
    "Creating a snapshot",
    "Preparing the update area",
    "Unpacking the new system",
    "Moving your accounts and settings",
    "Preparing the restart",
];

fn step(i: usize) {
    emit(&Event::Step { index: i as u32 + 1, total: STEPS.len() as u32, title: STEPS[i].into() });
}

fn progress(fraction: f64, detail: &str) {
    emit(&Event::Progress { fraction: fraction.clamp(0.0, 1.0), detail: detail.into() });
}

pub fn current_version() -> String {
    if let Some(v) = read_to_string(VERSION_FILE).map(|s| s.trim().to_string()).filter(|s| valid_version(s)) {
        return v;
    }
    read_to_string("/etc/os-release")
        .and_then(|t| t.lines().find_map(|l| l.strip_prefix("VERSION_ID=").map(|v| v.trim_matches('"').to_string())))
        .filter(|v| valid_version(v))
        .unwrap_or_else(|| "10.0".into())
}

fn free_bytes(path: &str) -> u64 {
    query("df", &["--output=avail", "-B1", path]).and_then(|o| o.lines().nth(1).and_then(|l| l.trim().parse().ok())).unwrap_or(0)
}

fn targets(manifest: &UpdateManifest) -> Vec<UpdateTarget> {
    let disks = probe::disks().unwrap_or_default();
    let model = |disk: &str| disks.iter().find(|d| d.path == disk).map(|d| d.model.clone()).unwrap_or_else(|| disk.to_string());
    let mut out = vec![];
    match snapshots::system_device() {
        Ok(device) => {
            let disk = probe::live_disk().unwrap_or_default();
            let free = free_bytes("/");
            let need = manifest.installed_size + HEADROOM;
            let problem = if Path::new(ENCRYPT_PENDING).exists() {
                Some("Disk encryption is still running. Install the update after it has finished.".to_string())
            } else if free < need {
                Some(format!("Not enough free space: the update needs {}, {} is free.", human_size(need), human_size(free)))
            } else {
                None
            };
            out.push(UpdateTarget {
                model: model(&disk),
                disk,
                encrypted: device.starts_with("/dev/mapper/") || device.starts_with("/dev/dm-"),
                device,
                running: true,
                free,
                problem,
            });
        }
        Err(_) => {}
    }
    // Other MAN installations: shown, but only the running one can be updated.
    let running_disk = out.first().map(|t| t.disk.clone()).unwrap_or_default();
    for inst in probe::installs(&disks) {
        if inst.disk == running_disk {
            continue;
        }
        out.push(UpdateTarget {
            model: model(&inst.disk),
            disk: inst.disk.clone(),
            device: inst.device.clone(),
            encrypted: inst.encrypted,
            running: false,
            free: 0,
            problem: Some("Start this MAN installation and open the update file there to update it.".into()),
        });
    }
    out
}

pub fn inspect(path: &str) -> Result<UpdateInfo> {
    let v = updfile::verify(path)?;
    let targets = targets(&v.manifest);
    Ok(UpdateInfo { targets, license: v.license, notes: v.notes, signed_by: v.signer, current_version: current_version(), manifest: v.manifest })
}

/// Stream the payload out of the .upd, hash it and unpack it into `dest`.
fn unpack(path: &str, m: &UpdateManifest, dest: &Path) -> Result<()> {
    let mut reader = Command::new("tar")
        .args(["-xOf", path, PAYLOAD])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("cannot read the update file")?;
    let mut unpacker = Command::new("tar")
        .args(["--zstd", "-xpf", "-", "--xattrs", "--xattrs-include=*", "--acls", "--numeric-owner", "-C"])
        .arg(dest)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("cannot start tar")?;
    let mut input = reader.stdout.take().unwrap();
    let mut output = unpacker.stdin.take().unwrap();
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    let mut done: u64 = 0;
    let mut last = Instant::now() - Duration::from_secs(1);
    let mut write_err = None;
    loop {
        let n = input.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        if write_err.is_none() {
            if let Err(e) = output.write_all(&buf[..n]) {
                write_err = Some(e);
            }
        }
        done += n as u64;
        if last.elapsed() > Duration::from_millis(400) {
            last = Instant::now();
            let f = if m.payload_size > 0 { done as f64 / m.payload_size as f64 } else { 0.0 };
            progress(f * 0.98, &format!("{} of {}", human_size(done), human_size(m.payload_size)));
        }
    }
    drop(output);
    let rs = reader.wait()?;
    let mut err = String::new();
    if let Some(mut e) = unpacker.stderr.take() {
        let _ = e.read_to_string(&mut err);
    }
    let us = unpacker.wait()?;
    for l in err.lines().take(20) {
        log(&format!("  {l}"));
    }
    if updfile::hex(&hasher.finalize()) != m.payload_sha256 {
        bail!("The update file is damaged (checksum mismatch). Download it again.");
    }
    if !rs.success() {
        bail!("The update file could not be read completely.");
    }
    if let Some(e) = write_err {
        bail!("Unpacking failed: {e}. {}", err.lines().last().unwrap_or(""));
    }
    if !us.success() {
        bail!("Unpacking the new system failed: {}", err.lines().last().unwrap_or("tar error"));
    }
    progress(1.0, "Unpacked");
    Ok(())
}

// ─────────────────────────── Carrying the machine over ───────────────────────────

/// Machine-specific files and folders kept from the running system.
const KEEP: &[&str] = &[
    "etc/fstab", "etc/crypttab", "etc/hostname", "etc/hosts", "etc/machine-id", "etc/localtime", "etc/timezone",
    "etc/default/keyboard", "etc/default/locale", "etc/locale.conf", "etc/vconsole.conf", "etc/adjtime",
    "etc/gdm3/daemon.conf", "etc/subuid", "etc/subgid", "etc/sudoers.d", "etc/NetworkManager/system-connections",
    "etc/dconf/db/local.d", "etc/man/snapshots.conf", "etc/cryptsetup-initramfs/conf-hook", "etc/sysusers.d/man.conf",
    "etc/tmpfiles.d/man.conf", "etc/systemd/system/emergency.service.d", "etc/systemd/system/rescue.service.d",
    "etc/ssh/ssh_host_rsa_key", "etc/ssh/ssh_host_rsa_key.pub", "etc/ssh/ssh_host_ecdsa_key", "etc/ssh/ssh_host_ecdsa_key.pub",
    "etc/ssh/ssh_host_ed25519_key", "etc/ssh/ssh_host_ed25519_key.pub",
    "var/lib/AccountsService", "var/lib/bluetooth", "var/lib/NetworkManager", "var/lib/flatpak", "var/lib/dbus/machine-id",
    "usr/local", "opt",
];

fn copy_over(new: &Path, rel: &str) -> Result<()> {
    let src = Path::new("/").join(rel);
    if std::fs::symlink_metadata(&src).is_err() {
        return Ok(());
    }
    let dst = new.join(rel);
    if let Ok(meta) = std::fs::symlink_metadata(&dst) {
        if meta.is_dir() && !meta.file_type().is_symlink() {
            std::fs::remove_dir_all(&dst)?;
        } else {
            std::fs::remove_file(&dst)?;
        }
    }
    std::fs::create_dir_all(dst.parent().unwrap())?;
    // Same Btrfs file system: reflinks make even large folders instant.
    run("cp", &["-a", "--reflink=auto", src.to_str().unwrap(), dst.to_str().unwrap()])?;
    Ok(())
}

fn read_db(path: &Path) -> Vec<String> {
    read_to_string(path).map(|t| t.lines().filter(|l| !l.trim().is_empty()).map(String::from).collect()).unwrap_or_default()
}

/// Accounts: the running system decides for every account it has; the new
/// system only adds accounts it introduces (new system users), never the
/// live session's accounts or anything colliding with an existing id.
fn merge_accounts(new: &Path) -> Result<()> {
    const LIVE_ONLY: &[&str] = &["MANInstaller", OOBE_USER];
    let id_of = |line: &str| line.split(':').nth(2).map(String::from).unwrap_or_default();
    for (file, has_id) in [("etc/passwd", true), ("etc/group", true), ("etc/shadow", false), ("etc/gshadow", false)] {
        let old = read_db(&Path::new("/").join(file));
        let fresh = read_db(&new.join(file));
        let name = |l: &String| l.split(':').next().unwrap_or("").to_string();
        let old_names: Vec<String> = old.iter().map(name).collect();
        let old_ids: Vec<String> = if has_id { old.iter().map(|l| id_of(l)).collect() } else { vec![] };
        let mut out = old.clone();
        for l in &fresh {
            let n = name(l);
            if old_names.contains(&n) || LIVE_ONLY.contains(&n.as_str()) {
                continue;
            }
            if has_id && old_ids.contains(&id_of(l)) {
                info(format!("Skipping new account {n}: its id is already used"));
                continue;
            }
            out.push(l.clone());
        }
        // shadow/gshadow entries only for accounts that exist.
        let target = new.join(file);
        let meta = std::fs::metadata(&target).ok();
        write_file(&target, &(out.join("\n") + "\n"), meta.map(|m| std::os::unix::fs::PermissionsExt::mode(&m.permissions()) & 0o7777).unwrap_or(0o644))?;
        if !has_id {
            let _ = run("chgrp", &["shadow", target.to_str().unwrap()]);
        }
    }
    Ok(())
}

fn migrate(new: &Path) -> Result<()> {
    for (i, rel) in KEEP.iter().enumerate() {
        progress(i as f64 / KEEP.len() as f64 * 0.8, &format!("/{rel}"));
        copy_over(new, rel)?;
    }
    progress(0.85, "Accounts");
    merge_accounts(new)?;
    // The live session's leftovers never belong on an installed system.
    let _ = std::fs::remove_file(new.join("etc/sudoers.d/90-maninstaller"));
    let _ = std::fs::remove_file(new.join("etc/polkit-1/rules.d/49-man-live.rules"));
    let _ = std::fs::remove_dir_all(new.join("home/MANInstaller"));
    for f in ["etc/skel/.face", "etc/skel/.face.icon"] {
        let _ = std::fs::remove_file(new.join(f));
    }
    Ok(())
}

// ─────────────────────────── Boot preparation ───────────────────────────

fn separate_boot() -> bool {
    query("findmnt", &["-n", "-o", "FSTYPE", "--mountpoint", "/boot"]).is_some()
}

fn running_kernel() -> Result<String> {
    Ok(run("uname", &["-r"])?.trim().to_string())
}

/// Inside the new system: purge installer-only packages, build initramfs
/// images and the boot menu it will use after the swap.
fn prepare_new_system(dev: &str, new: &Path, mounts: &mut Mounts) -> Result<()> {
    mounts.mount("/dev", &new.join("dev"), &["--bind"])?;
    mounts.mount("devpts", &new.join("dev/pts"), &["-t", "devpts"])?;
    mounts.mount("proc", &new.join("proc"), &["-t", "proc"])?;
    mounts.mount("sysfs", &new.join("sys"), &["-t", "sysfs"])?;
    mounts.mount("tmpfs", &new.join("run"), &["-t", "tmpfs"])?;
    if Path::new("/run/udev").is_dir() {
        mounts.mount("/run/udev", &new.join("run/udev"), &["--bind"])?;
    }
    mounts.mount(dev, &new.join(".snapshots"), &["-o", "subvol=@snapshots"])?;
    if separate_boot() {
        mounts.mount("/boot", &new.join("boot"), &["--bind"])?;
    }
    if Path::new("/boot/efi").is_dir() && query("findmnt", &["-n", "--mountpoint", "/boot/efi"]).is_some() {
        mounts.mount("/boot/efi", &new.join("boot/efi"), &["--bind"])?;
    }
    progress(0.1, "Removing installer-only software");
    if query("chroot", &[new.to_str().unwrap(), "dpkg-query", "-W", "-f=${Status}", "man-installer"]).is_some_and(|s| s.contains("installed")) {
        let _ = chroot(new, &["dpkg", "--purge", "man-installer"]);
    }
    progress(0.3, "Building the boot image (initramfs)");
    let kernels: Vec<String> = std::fs::read_dir(new.join("usr/lib/modules"))?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|v| new.join("boot").join(format!("vmlinuz-{v}")).exists())
        .collect();
    if kernels.is_empty() {
        bail!("The update contains no kernel.");
    }
    for v in &kernels {
        let mode = if new.join(format!("boot/initrd.img-{v}")).exists() { "-u" } else { "-c" };
        chroot(new, &["update-initramfs", mode, "-k", v])?;
    }
    progress(0.75, "Writing the boot menu");
    // GRUB seen from inside the new system calls it @update; after the swap
    // it is @system.
    let out = "/boot/grub/grub.cfg.man-update";
    chroot(new, &["grub-mkconfig", "-o", out])?;
    let cfg = new.join(out.trim_start_matches('/'));
    let text = std::fs::read_to_string(&cfg)?.replace("@update", "@system");
    std::fs::write(&cfg, text)?;
    if !separate_boot() {
        // Plain layout: the menu lives inside the subvolume itself.
        std::fs::rename(&cfg, new.join("boot/grub/grub.cfg"))?;
    }
    Ok(())
}

/// In the running system: a kernel + initramfs with the swap step, and a
/// one-time boot entry for them.
fn prepare_update_boot(id: &str, version: &str) -> Result<()> {
    let k = running_kernel()?;
    let _ = std::fs::remove_dir_all(UPDATE_BOOT);
    std::fs::create_dir_all(UPDATE_BOOT)?;
    std::fs::copy(format!("/boot/vmlinuz-{k}"), format!("{UPDATE_BOOT}/vmlinuz")).context("the running kernel was not found in /boot")?;
    progress(0.2, "Building the update boot image");
    run("mkinitramfs", &["-o", &format!("{UPDATE_BOOT}/initrd.img"), &k])?;
    write_file(STAGED, &format!("ID={id}\nVERSION={version}\n"), 0o644)?;
    progress(0.7, "Adding the one-time boot entry");
    run("update-grub", &[])?;
    run("grub-reboot", &["man-update"])?;
    Ok(())
}

fn discard_staged(top: &Path) {
    if top.join("@update").exists() {
        let _ = snapshots::delete_subvolume_tree(top, "@update");
    }
    let _ = std::fs::remove_file(STAGED);
    let _ = std::fs::remove_dir_all(UPDATE_BOOT);
}

pub fn stage(path: &str, device: &str, take_snapshot: bool) -> Result<String> {
    step(0);
    if query("id", &["-u"]).as_deref() != Some("0") {
        bail!("Updates must be installed by the system helper.");
    }
    let v = updfile::verify(path)?;
    let m = v.manifest;
    info(format!("Update {} {} (signed by {}), payload {}", m.name, m.version, v.signer, m.payload_sha256));
    let current = current_version();
    if version_cmp(&m.version, &current) != std::cmp::Ordering::Greater {
        bail!("MAN {} is not newer than the installed MAN {}.", m.version, current);
    }
    let target = targets(&m).into_iter().find(|t| t.device == device).ok_or_else(|| anyhow!("The chosen installation was not found."))?;
    if !target.running {
        bail!("Only the running MAN installation can be updated.");
    }
    if let Some(p) = target.problem {
        bail!("{p}");
    }
    let dev = snapshots::system_device()?;
    progress(1.0, "The update is genuine");

    step(1);
    if take_snapshot {
        let s = snapshots::create(&format!("Before update to MAN {}", m.version), "pre-update", false)?;
        info(format!("Snapshot {} created", s.id));
        progress(1.0, "Snapshot created");
    } else {
        progress(1.0, "Skipped");
    }

    step(2);
    let mut mounts = Mounts::default();
    let top = PathBuf::from(TOP);
    mounts.mount(&dev, &top, &["-o", "subvolid=5"])?;
    discard_staged(&top);
    run("btrfs", &["subvolume", "create", top.join("@update").to_str().unwrap()])?;
    progress(1.0, "Update area ready");

    let result = (|| -> Result<()> {
        step(3);
        unpack(path, &m, &top.join("@update"))?;

        step(4);
        let new = PathBuf::from(NEWROOT);
        let mut inner = Mounts::default();
        inner.mount(&dev, &new, &["-o", "subvol=@update"])?;
        migrate(&new)?;
        let id = format!("{}-{}", m.version, snapshot_id(now()));
        write_file(new.join(VERSION_FILE.trim_start_matches('/')), &format!("{}\n", m.version), 0o644)?;
        let info_json = serde_json::to_string(&LastUpdate { version: m.version.clone(), previous: current.clone(), date: rfc3339(now()) })?;
        write_file(new.join(INFO.trim_start_matches('/')), &info_json, 0o644)?;
        write_file(new.join("var/lib/man/update-ready"), &format!("{id}\n"), 0o644)?;
        progress(0.9, "Preparing the new system");
        prepare_new_system(&dev, &new, &mut inner)?;
        inner.release();

        step(5);
        prepare_update_boot(&id, &m.version)?;
        progress(1.0, "Ready");
        Ok(())
    })();
    if let Err(e) = result {
        info("Removing the incomplete update");
        mounts.release();
        let mut again = Mounts::default();
        if again.mount(&dev, &top, &["-o", "subvolid=5"]).is_ok() {
            discard_staged(&top);
        }
        let _ = run("update-grub", &[]);
        return Err(e);
    }
    mounts.release();
    Ok(format!("MAN {} is ready to install. Restart the computer to finish.", m.version))
}

/// New system, first start after the swap (man-update-finish.service).
pub fn finish() -> Result<()> {
    if !Path::new(APPLIED).exists() {
        return Ok(());
    }
    let _ = std::fs::remove_dir_all(UPDATE_BOOT);
    let _ = std::fs::remove_file("/boot/grub/grub.cfg.man-update");
    let _ = std::fs::remove_file(STAGED);
    if let Some(text) = read_to_string(INFO) {
        write_file(LAST_UPDATE, &text, 0o644)?;
        let _ = std::fs::remove_file(INFO);
    }
    std::fs::remove_file(APPLIED)?;
    if let Err(e) = run("update-grub", &[]) {
        info(format!("Warning: could not refresh the boot menu: {e:#}"));
    }
    info("Update finished.");
    Ok(())
}
