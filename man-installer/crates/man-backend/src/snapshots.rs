//! Snapshots of the running system. Layout on the Btrfs top level:
//!   @system      mounted at /
//!   @home        mounted at /home (never snapshotted or rolled back)
//!   @snapshots   mounted at /.snapshots, holding <id>/snapshot + <id>/info.json
use crate::util::*;
use anyhow::{anyhow, bail, Result};
use man_common::*;
use std::path::{Path, PathBuf};

const TOP: &str = "/run/man-snapshots/top";

pub fn write_info(dir: &Path, info: &SnapshotInfo) -> Result<()> {
    // One line, fixed key order: GRUB's menu generator reads it with sed.
    write_file(dir.join("info.json"), &(serde_json::to_string(info)? + "\n"), 0o644)
}

pub fn list() -> Vec<SnapshotInfo> {
    let mut v: Vec<SnapshotInfo> = std::fs::read_dir(SNAPSHOT_DIR)
        .map(|r| {
            r.filter_map(|e| e.ok())
                .filter(|e| e.path().join("snapshot").is_dir())
                .filter_map(|e| read_to_string(e.path().join("info.json")))
                .filter_map(|t| serde_json::from_str(&t).ok())
                .collect()
        })
        .unwrap_or_default();
    v.sort_by(|a, b| b.id.cmp(&a.id));
    v
}

/// Device and subvolume of a mount point, from findmnt.
fn mount_source(target: &str) -> Option<(String, String)> {
    let out = query("findmnt", &["-n", "-o", "SOURCE,FSTYPE,OPTIONS", "--mountpoint", target])?;
    let mut it = out.split_whitespace();
    let source = it.next()?;
    let fstype = it.next()?;
    let opts = it.next().unwrap_or("");
    if fstype != "btrfs" {
        return None;
    }
    let dev = source.split('[').next()?.to_string();
    let subvol = opts.split(',').find_map(|o| o.strip_prefix("subvol="))?.trim_start_matches('/').to_string();
    Some((dev, subvol))
}

/// The Btrfs device when / and /.snapshots follow the MAN layout.
pub fn system_device() -> Result<String> {
    let (dev, sub) = mount_source("/").ok_or_else(|| anyhow!("The system is not installed on Btrfs, so snapshots are not available."))?;
    if sub.starts_with("@system.old-") {
        bail!("A snapshot restore is waiting for a restart. Restart the computer to finish it first.");
    }
    if sub != "@system" {
        bail!("The running system is not the @system subvolume (it is {sub}). Snapshots are only managed on a normal MAN installation.");
    }
    match mount_source(SNAPSHOT_DIR) {
        Some((d, s)) if d == dev && s == "@snapshots" => Ok(dev),
        _ => bail!("{SNAPSHOT_DIR} is not mounted from the @snapshots subvolume."),
    }
}

fn update_grub() {
    if Path::new("/boot/grub/grub.cfg").exists() {
        if let Err(e) = run("update-grub", &[]) {
            info(format!("Warning: could not update the boot menu: {e:#}"));
        }
    }
}

fn unique_id() -> String {
    let mut ts = now();
    loop {
        let id = snapshot_id(ts);
        if !Path::new(SNAPSHOT_DIR).join(&id).exists() {
            return id;
        }
        ts += 1;
    }
}

pub fn create(description: &str, kind: &str, grub: bool) -> Result<SnapshotInfo> {
    system_device()?;
    let id = unique_id();
    let dir = Path::new(SNAPSHOT_DIR).join(&id);
    std::fs::create_dir_all(&dir)?;
    if let Err(e) = run("btrfs", &["subvolume", "snapshot", "-r", "/", dir.join("snapshot").to_str().unwrap()]) {
        let _ = std::fs::remove_dir(&dir);
        return Err(e);
    }
    let info = SnapshotInfo { id, created: rfc3339(now()), description: sanitize_description(description), kind: kind.into() };
    write_info(&dir, &info)?;
    if grub {
        update_grub();
    }
    Ok(info)
}

fn remove(id: &str) -> Result<()> {
    if !valid_snapshot_id(id) {
        bail!("Invalid snapshot id.");
    }
    let dir = Path::new(SNAPSHOT_DIR).join(id);
    if !dir.join("snapshot").is_dir() {
        bail!("The snapshot {id} does not exist.");
    }
    run("btrfs", &["subvolume", "delete", dir.join("snapshot").to_str().unwrap()])?;
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}

pub fn delete(id: &str) -> Result<()> {
    system_device()?;
    remove(id)?;
    update_grub();
    Ok(())
}

/// Keep only the newest `keep` snapshots of one automatic kind.
fn prune(kind: &str, keep: u32) {
    for old in list().into_iter().filter(|s| s.kind == kind).skip(keep as usize) {
        if let Err(e) = remove(&old.id) {
            info(format!("Warning: could not remove old snapshot {}: {e:#}", old.id));
        }
    }
}

pub fn config() -> SnapshotConfig {
    read_to_string(SNAPSHOT_CONFIG).map(|t| SnapshotConfig::parse(&t)).unwrap_or_default()
}

pub fn set_config(c: &SnapshotConfig) -> Result<()> {
    write_file(SNAPSHOT_CONFIG, &c.render(), 0o644)?;
    let verb = if c.daily { "enable" } else { "disable" };
    run("systemctl", &[verb, "--now", "man-snapshot-daily.timer"])?;
    Ok(())
}

/// Automatic snapshots: `daily` from the timer, `pre-update` from apt.
pub fn auto(kind: &str) -> Result<()> {
    if system_device().is_err() {
        return Ok(()); // live session or non-MAN layout: nothing to do
    }
    let c = config();
    let (enabled, keep, description) = match kind {
        "daily" => (c.daily, c.keep_daily, "Daily snapshot"),
        "pre-update" => (c.pre_update, c.keep_pre_update, "Before software update"),
        _ => bail!("unknown automatic snapshot kind {kind}"),
    };
    if !enabled {
        return Ok(());
    }
    if kind == "pre-update" {
        // One snapshot per burst of apt runs is enough.
        let recent = list().into_iter().find(|s| s.kind == kind).map(|s| s.id).unwrap_or_default();
        if recent >= snapshot_id(now().saturating_sub(600)) {
            return Ok(());
        }
    }
    create(description, kind, false)?;
    prune(kind, keep);
    update_grub();
    Ok(())
}

/// Delete a subvolume and every subvolume nested inside it.
pub fn delete_subvolume_tree(top: &Path, name: &str) -> Result<()> {
    let out = run("btrfs", &["subvolume", "list", "-o", top.join(name).to_str().unwrap()])?;
    let mut nested: Vec<String> = out.lines().filter_map(|l| l.split(" path ").nth(1).map(|p| p.trim().to_string())).collect();
    nested.sort_by_key(|p| std::cmp::Reverse(p.len()));
    for p in nested {
        // Paths may be relative to the top level or prefixed with <FS_TREE>/.
        let p = p.trim_start_matches("<FS_TREE>/");
        run("btrfs", &["subvolume", "delete", top.join(p).to_str().unwrap()])?;
    }
    run("btrfs", &["subvolume", "delete", top.join(name).to_str().unwrap()])?;
    Ok(())
}

/// Replace @system with a writable copy of a snapshot. Takes effect on the
/// next boot; the current system is kept as a snapshot first.
pub fn rollback(id: &str) -> Result<String> {
    let dev = system_device()?;
    if !valid_snapshot_id(id) {
        bail!("Invalid snapshot id.");
    }
    let target = list().into_iter().find(|s| s.id == id).ok_or_else(|| anyhow!("The snapshot {id} does not exist."))?;
    let safety = create(&format!("Before restoring {}", target.description), "pre-rollback", false)?;
    info(format!("Saved the current system as snapshot {}", safety.id));

    let mut mounts = Mounts::default();
    let top = PathBuf::from(TOP);
    mounts.mount(&dev, &top, &["-o", "subvolid=5"])?;
    let old = format!("@system.old-{}", snapshot_id(now()));
    std::fs::rename(top.join("@system"), top.join(&old))?;
    let restored = run(
        "btrfs",
        &["subvolume", "snapshot", top.join("@snapshots").join(id).join("snapshot").to_str().unwrap(), top.join("@system").to_str().unwrap()],
    );
    if let Err(e) = restored {
        // Put the current system back so nothing changes.
        let _ = std::fs::rename(top.join(&old), top.join("@system"));
        return Err(e);
    }
    write_file(top.join("@system/var/lib/man/rollback-done"), &format!("{id}\n"), 0o644)?;
    if let Err(e) = prepare_boot_menu(&dev, &top) {
        info(format!("Warning: could not refresh the boot menu of the restored system: {e:#}"));
    }
    mounts.release();
    Ok(format!("The system will be restored to “{}” when you restart.", target.description))
}

/// GRUB reads grub.cfg from inside @system (unless /boot is a separate
/// partition). Make sure the restored system has one that matches it.
fn prepare_boot_menu(dev: &str, top: &Path) -> Result<()> {
    if query("findmnt", &["-n", "-o", "FSTYPE", "--mountpoint", "/boot"]).is_some() {
        return Ok(()); // separate /boot: its grub.cfg is not part of the snapshot
    }
    let cfg = top.join("@system/boot/grub/grub.cfg");
    if !cfg.exists() && Path::new("/boot/grub/grub.cfg").exists() {
        // Safety net: never leave GRUB without a menu.
        std::fs::create_dir_all(cfg.parent().unwrap())?;
        std::fs::copy("/boot/grub/grub.cfg", &cfg)?;
    }
    let root = PathBuf::from("/run/man-snapshots/restored");
    let mut mounts = Mounts::default();
    mounts.mount(dev, &root, &["-o", "subvol=@system"])?;
    mounts.mount(dev, &root.join(".snapshots"), &["-o", "subvol=@snapshots"])?;
    mounts.mount("/dev", &root.join("dev"), &["--bind"])?;
    mounts.mount("proc", &root.join("proc"), &["-t", "proc"])?;
    mounts.mount("sysfs", &root.join("sys"), &["-t", "sysfs"])?;
    mounts.mount("tmpfs", &root.join("run"), &["-t", "tmpfs"])?;
    chroot(&root, &["update-grub"])?;
    Ok(())
}

/// Runs at boot: drop systems replaced by a rollback and refresh the menu.
pub fn boot_cleanup() -> Result<()> {
    // Only after a healthy boot: in emergency or maintenance mode the
    // replaced system may still be needed to recover.
    // is-system-running exits non-zero for "degraded", so read stdout directly.
    let state = std::process::Command::new("systemctl")
        .arg("is-system-running")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    if !(state == "running" || state == "degraded") {
        info(format!("Skipping boot cleanup: system state is '{state}'"));
        return Ok(());
    }
    let dev = match system_device() {
        Ok(d) => d,
        Err(_) => return Ok(()),
    };
    let mut mounts = Mounts::default();
    let top = PathBuf::from(TOP);
    mounts.mount(&dev, &top, &["-o", "subvolid=5"])?;
    let mut changed = false;
    for e in std::fs::read_dir(&top)?.filter_map(|e| e.ok()) {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with("@system.old-") {
            match delete_subvolume_tree(&top, &name) {
                Ok(()) => changed = true,
                Err(err) => info(format!("Warning: could not remove {name}: {err:#}")),
            }
        }
    }
    mounts.release();
    let marker = Path::new("/var/lib/man/rollback-done");
    if marker.exists() {
        let _ = std::fs::remove_file(marker);
        changed = true;
    }
    if changed {
        update_grub();
    }
    Ok(())
}
