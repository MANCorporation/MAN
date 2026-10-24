use crate::util::{query, read_to_string, run, Mounts};
use anyhow::Result;
use man_common::*;
use serde_json::Value;
use std::path::Path;

const LSBLK_COLS: &str = "NAME,PATH,TYPE,SIZE,MODEL,SERIAL,TRAN,RM,RO,FSTYPE,LABEL,UUID,PARTTYPE,MOUNTPOINTS,PKNAME";

fn s(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(|x| x.as_str()).map(|x| x.trim().to_string()).filter(|x| !x.is_empty())
}

fn b(v: &Value, k: &str) -> bool {
    match v.get(k) {
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => s == "1",
        Some(Value::Number(n)) => n.as_u64() == Some(1),
        _ => false,
    }
}

fn mountpoints(v: &Value) -> Vec<String> {
    v.get("mountpoints")
        .and_then(|m| m.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default()
}

/// Any mounted filesystem or active swap anywhere below this lsblk node.
fn subtree_busy(v: &Value) -> bool {
    !mountpoints(v).is_empty() || v.get("children").and_then(|c| c.as_array()).map(|c| c.iter().any(subtree_busy)).unwrap_or(false)
}

/// The live medium is a partition DUHACH! made on an internal disk (next to
/// Windows), not a USB stick or DVD: then only that partition is off limits.
pub fn installer_partition() -> Option<String> {
    let src = query("findmnt", &["-n", "-o", "SOURCE", "--mountpoint", "/run/live/medium"])?;
    let info = query("lsblk", &["-ndo", "TYPE,FSTYPE,TRAN,RM", &src]).unwrap_or_default();
    let disk = query("lsblk", &["-ndo", "PKNAME", &src]).unwrap_or_default();
    let disk_info = query("lsblk", &["-ndo", "TRAN,RM", &format!("/dev/{disk}")]).unwrap_or_default();
    let fields: Vec<&str> = info.split_whitespace().collect();
    let removable = disk_info.split_whitespace().any(|w| w == "usb" || w == "1");
    (fields.first() == Some(&"part") && fields.get(1) == Some(&"vfat") && !removable).then_some(src)
}

/// The disk holding the running root filesystem (or the live medium).
pub fn live_disk() -> Option<String> {
    let mut sources = vec![];
    let installer = installer_partition();
    for target in ["/run/live/medium", "/"] {
        if let Some(src) = query("findmnt", &["-n", "-o", "SOURCE", "--target", target]) {
            // Btrfs reports "/dev/vda2[/@system]".
            let src = src.split('[').next().unwrap_or("").to_string();
            // A DUHACH! installer partition does not make its disk the live disk.
            if installer.as_deref() != Some(src.as_str()) {
                sources.push(src);
            }
        }
    }
    for src in sources.into_iter().filter(|s| s.starts_with("/dev/")) {
        // Inverse tree: the device itself, then partitions/LUKS layers below
        // it, down to the disk.
        let tree = query("lsblk", &["-snlo", "PATH,TYPE", &src]).unwrap_or_default();
        if let Some(disk) = tree.lines().find_map(|l| {
            let mut it = l.split_whitespace();
            let (path, kind) = (it.next()?, it.next()?);
            (kind == "disk").then(|| path.to_string())
        }) {
            return Some(disk);
        }
    }
    None
}

/// Unpartitioned regions of a GPT disk, aligned to 1 MiB, at least 1 GiB.
pub fn free_space(disk: &str) -> Vec<FreeSpace> {
    const MIB: u64 = 1 << 20;
    let out = query("parted", &["-s", "-m", disk, "unit", "B", "print", "free"]).unwrap_or_default();
    if !out.lines().nth(1).is_some_and(|l| l.contains(":gpt:")) {
        return vec![];
    }
    let mut v: Vec<FreeSpace> = out
        .lines()
        .filter(|l| l.trim_end().ends_with("free;"))
        .filter_map(|l| {
            let f: Vec<&str> = l.split(':').collect();
            let num = |x: &str| x.trim_end_matches('B').parse::<u64>().ok();
            let (start, end) = (num(f.get(1)?)?, num(f.get(2)?)? + 1);
            let (start, end) = (start.div_ceil(MIB) * MIB, end / MIB * MIB);
            (end > start && end - start >= 1 << 30).then_some(FreeSpace { start, size: end - start })
        })
        .collect();
    v.sort_by_key(|f| std::cmp::Reverse(f.size));
    v
}

pub fn disks() -> Result<Vec<Disk>> {
    let out = run("lsblk", &["-J", "-b", "-o", LSBLK_COLS])?;
    let json: Value = serde_json::from_str(&out)?;
    let live = live_disk();
    let mut disks = vec![];
    for d in json["blockdevices"].as_array().cloned().unwrap_or_default() {
        if s(&d, "type").as_deref() != Some("disk") {
            continue;
        }
        let path = s(&d, "path").unwrap_or_default();
        // Skip zram, loop and optical devices.
        if path.starts_with("/dev/zram") || path.starts_with("/dev/loop") || path.starts_with("/dev/sr") {
            continue;
        }
        let mut parts = vec![];
        for p in d.get("children").and_then(|c| c.as_array()).cloned().unwrap_or_default() {
            if s(&p, "type").as_deref() != Some("part") {
                continue;
            }
            let parttype = s(&p, "parttype").map(|t| t.to_lowercase());
            let mut mps = mountpoints(&p);
            for c in p.get("children").and_then(|c| c.as_array()).cloned().unwrap_or_default() {
                mps.extend(mountpoints(&c));
            }
            parts.push(Partition {
                path: s(&p, "path").unwrap_or_default(),
                disk: path.clone(),
                size: p["size"].as_u64().unwrap_or(0),
                fstype: s(&p, "fstype"),
                label: s(&p, "label"),
                uuid: s(&p, "uuid"),
                is_esp: parttype.as_deref() == Some(ESP_GUID),
                parttype,
                mountpoints: mps,
            });
        }
        let removable = b(&d, "rm") || s(&d, "tran").as_deref() == Some("usb");
        let windows = parts.iter().any(|p| p.fstype.as_deref() == Some("ntfs") || p.parttype.as_deref() == Some(MSR_GUID));
        disks.push(Disk {
            // The disk running MAN, or the live medium it was started from. A
            // DUHACH! installer partition (FAT, on an internal disk) only
            // protects itself, so MAN can go next to Windows on that disk.
            is_live: live.as_deref() == Some(path.as_str())
                || parts.iter().any(|p| p.label.as_deref() == Some(LIVE_LABEL) && (removable || p.fstype.as_deref() != Some("vfat"))),
            free: free_space(&path),
            windows,
            busy: subtree_busy(&d),
            model: s(&d, "model").unwrap_or_else(|| {
                match s(&d, "tran").as_deref() {
                    Some("nvme") => "NVMe Drive",
                    Some("usb") => "USB Drive",
                    Some("sata") | Some("ata") => "SATA Drive",
                    _ if path.starts_with("/dev/vd") => "Virtual Disk",
                    _ => "Disk",
                }
                .into()
            }),
            serial: s(&d, "serial").unwrap_or_default(),
            size: d["size"].as_u64().unwrap_or(0),
            transport: s(&d, "tran").unwrap_or_default(),
            removable: b(&d, "rm"),
            read_only: b(&d, "ro"),
            partitions: parts,
            path,
        });
    }
    Ok(disks)
}

/// Inspect a (possibly unlocked) MAN Btrfs filesystem read-only.
pub fn inspect_btrfs(device: &str, install: &mut ExistingInstall) -> Result<()> {
    let mut mounts = Mounts::default();
    let top = Path::new("/run/man-installer/inspect");
    mounts.mount(device, top, &["-o", "ro,subvolid=5"])?;
    install.has_system = top.join("@system").is_dir();
    install.has_home = top.join("@home").is_dir();
    if let Ok(entries) = std::fs::read_dir(top.join("@home")) {
        install.home_dirs = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n != "lost+found")
            .collect();
        install.home_dirs.sort();
    }
    install.snapshots = std::fs::read_dir(top.join("@snapshots"))
        .map(|r| r.filter_map(|e| e.ok()).filter(|e| e.path().join("info.json").is_file()).count())
        .unwrap_or(0);
    install.inspected = true;
    Ok(())
}

pub fn installs(disks: &[Disk]) -> Vec<ExistingInstall> {
    let mut found = vec![];
    for d in disks.iter().filter(|d| !d.is_live) {
        let esp = d.partitions.iter().find(|p| p.is_esp).map(|p| p.path.clone());
        let boot = d
            .partitions
            .iter()
            .find(|p| p.label.as_deref() == Some("MAN_BOOT") && p.fstype.as_deref() == Some("ext4"))
            .map(|p| p.path.clone());
        for p in &d.partitions {
            let label = p.label.as_deref();
            let fstype = p.fstype.as_deref();
            let encrypted = fstype == Some("crypto_LUKS") && label == Some(LUKS_LABEL);
            let plain = fstype == Some("btrfs") && label == Some(BTRFS_LABEL);
            if !(encrypted || plain) || !p.mountpoints.is_empty() {
                continue;
            }
            let mut inst = ExistingInstall {
                disk: d.path.clone(),
                device: p.path.clone(),
                encrypted,
                esp: esp.clone(),
                boot: if encrypted { boot.clone() } else { None },
                ..Default::default()
            };
            if plain {
                if let Err(e) = inspect_btrfs(&p.path, &mut inst) {
                    crate::util::log(&format!("cannot inspect {}: {e:#}", p.path));
                    continue;
                }
                if !inst.has_system && !inst.has_home {
                    continue;
                }
            }
            found.push(inst);
        }
    }
    found
}

pub fn inspect_encrypted(device: &str, passphrase: &str) -> Result<ExistingInstall> {
    let all = disks()?;
    let mut inst = installs(&all)
        .into_iter()
        .find(|i| i.device == device && i.encrypted)
        .ok_or_else(|| anyhow::anyhow!("{device} is not an encrypted MAN installation."))?;
    let mut mounts = Mounts::default();
    let mapper = mounts.luks_open(device, "man_inspect", passphrase, true)?;
    inspect_btrfs(&mapper, &mut inst)?;
    Ok(inst)
}

pub fn source_bytes() -> u64 {
    // Started from the ISO, the root is a RAM overlay: the medium records the
    // system's size.
    if let Some(n) = read_to_string("/run/live/medium/live/filesystem.size").and_then(|s| s.trim().parse().ok()) {
        return n;
    }
    // Used bytes of the root filesystem, the bulk of what gets copied.
    query("df", &["--output=used", "-B1", "/"])
        .and_then(|o| o.lines().nth(1).and_then(|l| l.trim().parse().ok()))
        .unwrap_or(8 << 30)
}

pub fn current_timezone() -> String {
    std::fs::read_link("/etc/localtime")
        .ok()
        .and_then(|p| p.to_str().and_then(|s| s.split("zoneinfo/").nth(1).map(String::from)))
        .or_else(|| read_to_string("/etc/timezone").map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Etc/UTC".into())
}

pub fn current_keyboard() -> String {
    read_to_string("/etc/default/keyboard")
        .and_then(|t| {
            t.lines()
                .find_map(|l| l.strip_prefix("XKBLAYOUT=").map(|v| v.trim_matches('"').split(',').next().unwrap_or("us").to_string()))
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "us".into())
}

pub fn probe() -> Result<Probe> {
    let disks = disks()?;
    Ok(Probe {
        uefi: Path::new("/sys/firmware/efi").is_dir(),
        live_disk: live_disk(),
        installs: installs(&disks),
        disks,
        source_bytes: source_bytes(),
        timezone: current_timezone(),
        keyboard: current_keyboard(),
        language: crate::locale::current_language(Path::new("/")),
    })
}
