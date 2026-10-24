use crate::probe;
use crate::snapshots::{delete_subvolume_tree, write_info};
use crate::util::*;
use anyhow::{anyhow, bail, Context, Result};
use man_common::*;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const TOP: &str = "/run/man-installer/top";
const TARGET: &str = "/run/man-installer/target";
const MAPPER: &str = "man_root";
const BTRFS_OPTS: &str = "compress=zstd:1,noatime";

const STEPS: [&str; 7] = [
    "Checking the installation plan",
    "Preparing the disk",
    "Copying MAN to the disk",
    "Configuring the new system",
    "Installing the bootloader",
    "Creating the first snapshot",
    "Finishing up",
];

fn step(i: usize) {
    emit(&Event::Step { index: i as u32 + 1, total: STEPS.len() as u32, title: STEPS[i].into() });
}

fn progress(fraction: f64, detail: &str) {
    emit(&Event::Progress { fraction: fraction.clamp(0.0, 1.0), detail: detail.into() });
}

/// Paths never copied from the live system.
const EXCLUDES: &[&str] = &[
    "/dev/*", "/proc/*", "/sys/*", "/run/*", "/tmp/*", "/var/tmp/*", "/mnt/*", "/media/*",
    "/lost+found", "/home/*", "/root/*", "/.snapshots/*", "/swapfile", "/boot/efi/*",
    "/var/cache/apt/archives/*.deb", "/var/log/journal/*", "/var/lib/systemd/random-seed",
    "/var/lib/AccountsService/users/MANInstaller", "/var/lib/gdm3/.cache/*",
];

struct Layout {
    esp: String,
    boot: Option<String>,
    /// Partition holding Btrfs or the LUKS container.
    root_part: String,
    /// Device carrying Btrfs (the mapper when encrypted).
    btrfs: String,
}

fn part_path(disk: &str, n: u32) -> String {
    if disk.chars().last().is_some_and(|c| c.is_ascii_digit()) { format!("{disk}p{n}") } else { format!("{disk}{n}") }
}

fn uuid_of(dev: &str) -> Result<String> {
    query("blkid", &["-s", "UUID", "-o", "value", dev])
        .filter(|u| !u.is_empty())
        .ok_or_else(|| anyhow!("cannot read the UUID of {dev}"))
}

fn ensure_unused(p: &Partition, live: Option<&str>) -> Result<()> {
    if p.label.as_deref() == Some(LIVE_LABEL) {
        bail!("{} is the boot partition of the MAN live medium.", p.path);
    }
    if !p.mountpoints.is_empty() {
        bail!("{} is in use ({}). Unmount it first.", p.path, p.mountpoints.join(", "));
    }
    if live == Some(p.disk.as_str()) {
        bail!("{} belongs to the disk MAN is running from.", p.path);
    }
    Ok(())
}

fn find_partition(disks: &[Disk], path: &str) -> Result<Partition> {
    disks
        .iter()
        .flat_map(|d| d.partitions.iter())
        .find(|p| p.path == path)
        .cloned()
        .ok_or_else(|| anyhow!("The partition {path} no longer exists."))
}

fn validate(plan: &Plan, source: u64) -> Result<(Disk, Vec<Disk>)> {
    if query("id", &["-u"]).as_deref() != Some("0") {
        bail!("The installer backend must run as root.");
    }
    if !Path::new("/sys/firmware/efi").is_dir() {
        bail!("This computer was not started in UEFI mode. MAN requires UEFI.");
    }
    if let Some(u) = &plan.user {
        if !valid_username(&u.username) {
            bail!("The username is not valid.");
        }
        if u.password.is_empty() {
            bail!("A password is required.");
        }
    }
    if !valid_hostname(&plan.hostname) {
        bail!("The computer name is not valid.");
    }
    if plan.timezone.contains("..") || !Path::new("/usr/share/zoneinfo").join(&plan.timezone).is_file() {
        bail!("Unknown time zone {}.", plan.timezone);
    }
    if plan.language.as_deref().is_some_and(|l| !valid_locale(l)) {
        bail!("Unknown language.");
    }
    if plan.keyboard.is_empty() || !plan.keyboard.chars().all(|c| c.is_ascii_alphanumeric() || "_+-".contains(c)) {
        bail!("Unknown keyboard layout {}.", plan.keyboard);
    }
    if plan.encrypt && plan.passphrase.as_deref().unwrap_or("").len() < 8 && plan.mode != Mode::Reinstall {
        bail!("The encryption passphrase must have at least 8 characters.");
    }
    let disks = probe::disks()?;
    let disk = disks.iter().find(|d| d.path == plan.disk).cloned().ok_or_else(|| anyhow!("The disk {} is no longer present.", plan.disk))?;
    if disk.is_live {
        bail!("{} is the disk MAN is running from and cannot be used.", disk.path);
    }
    if disk.read_only {
        bail!("{} is read-only.", disk.path);
    }
    if plan.confirm != confirm_token(&disk) {
        bail!("The disk changed after you confirmed the installation. Nothing was modified; please review and confirm again.");
    }
    let needed = source + HEADROOM;
    match plan.mode {
        Mode::Erase => {
            if disk.busy {
                bail!("A partition on {} is in use. Unmount it first.", disk.path);
            }
            let overhead = 2u64 << 30; // EFI + boot partitions
            if disk.size < needed + overhead {
                bail!("{} is too small: MAN needs at least {}.", disk.path, human_size(needed + overhead));
            }
        }
        Mode::Manual => {
            let live = probe::live_disk();
            let esp = find_partition(&disks, plan.esp.as_deref().ok_or_else(|| anyhow!("Choose an EFI system partition."))?)?;
            let root = find_partition(&disks, plan.root.as_deref().ok_or_else(|| anyhow!("Choose a system partition."))?)?;
            ensure_unused(&esp, live.as_deref())?;
            ensure_unused(&root, live.as_deref())?;
            if esp.path == root.path {
                bail!("The EFI and system partitions must be different.");
            }
            if root.disk != disk.path {
                bail!("The confirmed disk does not hold the chosen system partition.");
            }
            if !plan.format_esp && esp.fstype.as_deref() != Some("vfat") {
                bail!("{} is not FAT formatted; enable formatting of the EFI partition.", esp.path);
            }
            if root.size < needed {
                bail!("{} is too small: MAN needs at least {}.", root.path, human_size(needed));
            }
            if plan.encrypt {
                let boot = find_partition(&disks, plan.boot.as_deref().ok_or_else(|| anyhow!("Encryption needs a separate boot partition."))?)?;
                ensure_unused(&boot, live.as_deref())?;
                if boot.path == root.path || boot.path == esp.path {
                    bail!("The boot partition must be separate from the EFI and system partitions.");
                }
                if boot.size < 512 << 20 {
                    bail!("The boot partition must be at least 512 MB.");
                }
            }
        }
        Mode::Alongside => {
            let free = disk.largest_free().ok_or_else(|| anyhow!("{} has no free space next to the other systems.", disk.path))?;
            if free.size < needed + BOOT_SIZE {
                bail!("The free space on {} ({}) is too small: MAN needs at least {}.", disk.path, human_size(free.size), human_size(needed + BOOT_SIZE));
            }
            let esp = esp_of(&disk)?;
            if !esp.mountpoints.is_empty() {
                bail!("The EFI system partition {} is in use. Unmount it first.", esp.path);
            }
        }
        Mode::Reinstall => {
            let root = plan.root.as_deref().ok_or_else(|| anyhow!("No installation chosen."))?;
            let part = find_partition(&disks, root)?;
            ensure_unused(&part, probe::live_disk().as_deref())?;
            if part.disk != disk.path {
                bail!("The confirmed disk does not hold the chosen installation.");
            }
        }
    }
    Ok((disk, disks))
}

fn settle(paths: &[&str]) -> Result<()> {
    let _ = Command::new("udevadm").arg("settle").status();
    let deadline = Instant::now() + Duration::from_secs(15);
    while paths.iter().any(|p| !Path::new(p).exists()) {
        if Instant::now() > deadline {
            bail!("The new partitions did not appear: {}", paths.join(", "));
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Ok(())
}

fn partition_disk(plan: &Plan, disk: &Disk, mounts: &mut Mounts) -> Result<Layout> {
    progress(0.05, "Removing old partition signatures");
    for p in &disk.partitions {
        run("wipefs", &["-a", &p.path])?;
    }
    run("wipefs", &["-a", &disk.path])?;
    run("sgdisk", &["--zap-all", &disk.path])?;
    progress(0.25, "Creating partitions");
    let root_type = if plan.encrypt { "8309" } else { "8304" };
    let mut args = vec!["-n1:1MiB:+1GiB", "-t1:ef00", "-c1:MAN_EFI"];
    // A separate /boot is always created: it lets MAN Setup encrypt the
    // system later, since GRUB then never needs to read encrypted data.
    args.extend(["-n2:0:+1GiB", "-t2:8300", "-c2:MAN_BOOT"]);
    let root_n = 3;
    let new_root = format!("-n{root_n}:0:0");
    let typ = format!("-t{root_n}:{root_type}");
    let name = format!("-c{root_n}:MAN_ROOT");
    args.extend([new_root.as_str(), typ.as_str(), name.as_str(), disk.path.as_str()]);
    run("sgdisk", &args)?;
    let _ = run("partprobe", &[&disk.path]);
    let esp = part_path(&disk.path, 1);
    let root = part_path(&disk.path, root_n);
    let boot = Some(part_path(&disk.path, 2));
    let mut all = vec![esp.as_str(), root.as_str()];
    if let Some(b) = &boot {
        all.push(b);
    }
    settle(&all)?;
    progress(0.45, "Formatting the EFI system partition");
    run("mkfs.fat", &["-F", "32", "-n", "MAN_EFI", &esp])?;
    format_system(plan, &root, boot.as_deref(), mounts).map(|btrfs| Layout { esp, boot, root_part: root, btrfs })
}

/// Create LUKS (optional), the boot filesystem (with LUKS) and Btrfs.
fn format_system(plan: &Plan, root: &str, boot: Option<&str>, mounts: &mut Mounts) -> Result<String> {
    let btrfs = if plan.encrypt {
        progress(0.55, "Encrypting the system partition");
        let pass = plan.passphrase.as_deref().unwrap_or_default();
        run_input(
            "cryptsetup",
            &["luksFormat", "--type", "luks2", "--batch-mode", "--label", LUKS_LABEL, "--key-file=-", root],
            Some(pass.as_bytes()),
        )?;
        mounts.luks_open(root, MAPPER, pass, false)?
    } else {
        root.to_string()
    };
    if let Some(b) = boot {
        progress(0.7, "Formatting the boot partition");
        run("mkfs.ext4", &["-F", "-q", "-L", "MAN_BOOT", b])?;
    }
    progress(0.8, "Creating the Btrfs file system");
    run("mkfs.btrfs", &["-f", "-L", BTRFS_LABEL, &btrfs])?;
    Ok(btrfs)
}

fn prepare_manual(plan: &Plan, mounts: &mut Mounts) -> Result<Layout> {
    let esp = plan.esp.clone().unwrap();
    let root = plan.root.clone().unwrap();
    let boot = if plan.encrypt { plan.boot.clone() } else { None };
    progress(0.1, "Removing old file system signatures");
    run("wipefs", &["-a", &root])?;
    if let Some(b) = &boot {
        run("wipefs", &["-a", b])?;
    }
    if plan.format_esp {
        progress(0.3, "Formatting the EFI system partition");
        run("wipefs", &["-a", &esp])?;
        run("mkfs.fat", &["-F", "32", "-n", "MAN_EFI", &esp])?;
    }
    let btrfs = format_system(plan, &root, boot.as_deref(), mounts)?;
    Ok(Layout { esp, boot, root_part: root, btrfs })
}

/// The EFI system partition already on a disk (Windows'), kept as it is.
fn esp_of(disk: &Disk) -> Result<Partition> {
    disk.partitions
        .iter()
        .find(|p| p.is_esp && p.fstype.as_deref() == Some("vfat"))
        .cloned()
        .ok_or_else(|| anyhow!("{} has no EFI system partition to share.", disk.path))
}

/// Next to Windows: /boot and the system partition go into the largest free
/// region; the existing EFI partition is shared and nothing else changes.
fn prepare_alongside(plan: &Plan, disk: &Disk, mounts: &mut Mounts) -> Result<Layout> {
    let free = disk.largest_free().ok_or_else(|| anyhow!("No free space on {}", disk.path))?;
    let esp = esp_of(disk)?.path;
    let used: Vec<u32> = disk.partitions.iter().filter_map(|p| partition_number(&p.path)).collect();
    let mut numbers = (1..=128u32).filter(|n| !used.contains(n));
    let (boot_n, root_n) = (numbers.next().unwrap(), numbers.next().unwrap());
    let sector = 512u64;
    let boot_start = free.start / sector;
    let root_start = (free.start + BOOT_SIZE) / sector;
    let root_end = (free.start + free.size) / sector - 1;
    progress(0.2, &format!("Creating partitions in {} of free space", human_size(free.size)));
    let root_type = if plan.encrypt { "8309" } else { "8304" };
    run(
        "sgdisk",
        &[
            &format!("-n{boot_n}:{boot_start}:{}", root_start - 1),
            &format!("-t{boot_n}:8300"),
            &format!("-c{boot_n}:MAN_BOOT"),
            &format!("-n{root_n}:{root_start}:{root_end}"),
            &format!("-t{root_n}:{root_type}"),
            &format!("-c{root_n}:MAN_ROOT"),
            &disk.path,
        ],
    )?;
    // The disk is in use (the installer partition is mounted): tell the
    // kernel about the two new partitions only.
    for n in [boot_n, root_n] {
        let _ = run("partx", &["-a", "--nr", &n.to_string(), &disk.path]);
    }
    let boot = part_path(&disk.path, boot_n);
    let root = part_path(&disk.path, root_n);
    settle(&[boot.as_str(), root.as_str()])?;
    let btrfs = format_system(plan, &root, Some(&boot), mounts)?;
    Ok(Layout { esp, boot: Some(boot), root_part: root, btrfs })
}

fn partition_number(path: &str) -> Option<u32> {
    let digits: String = path.chars().rev().take_while(|c| c.is_ascii_digit()).collect();
    digits.chars().rev().collect::<String>().parse().ok()
}

fn prepare_reinstall(plan: &Plan, disks: &[Disk], mounts: &mut Mounts) -> Result<Layout> {
    let root = plan.root.clone().unwrap();
    let installs = probe::installs(disks);
    let inst = installs.iter().find(|i| i.device == root).ok_or_else(|| anyhow!("{root} is not a MAN installation."))?;
    let esp = plan.esp.clone().or_else(|| inst.esp.clone()).ok_or_else(|| anyhow!("No EFI system partition found next to {root}."))?;
    let btrfs = if inst.encrypted {
        progress(0.1, "Unlocking the encrypted installation");
        mounts.luks_open(&root, MAPPER, plan.passphrase.as_deref().unwrap_or_default(), false)?
    } else {
        root.clone()
    };
    let boot = if inst.encrypted { inst.boot.clone() } else { None };
    if inst.encrypted && boot.is_none() {
        bail!("The encrypted installation has no MAN_BOOT partition.");
    }
    if let Some(b) = &boot {
        progress(0.2, "Formatting the boot partition");
        // Keep the UUID: snapshots of the previous system mount /boot by it,
        // and must still boot after the reinstall.
        let uuid = uuid_of(b)?;
        run("mkfs.ext4", &["-F", "-q", "-L", "MAN_BOOT", "-U", &uuid, b])?;
    }
    Ok(Layout { esp, boot, root_part: root, btrfs })
}

/// Set up @system/@home/@snapshots. On reinstall the old @system is kept as
/// a read-only snapshot, so the reinstall itself can be rolled back.
fn create_subvolumes(layout: &Layout, reinstall: bool, mounts: &mut Mounts) -> Result<()> {
    let top = Path::new(TOP);
    mounts.mount(&layout.btrfs, top, &["-o", &format!("subvolid=5,{BTRFS_OPTS}")])?;
    let sys = top.join("@system");
    if reinstall && sys.is_dir() {
        progress(0.5, "Saving the current system as a snapshot");
        let id = snapshot_id(now());
        let dir = top.join("@snapshots").join(&id);
        std::fs::create_dir_all(&dir)?;
        run("btrfs", &["subvolume", "snapshot", "-r", sys.to_str().unwrap(), dir.join("snapshot").to_str().unwrap()])?;
        write_info(&dir, &SnapshotInfo { id, created: rfc3339(now()), description: "System before reinstalling".into(), kind: "pre-reinstall".into() })?;
        progress(0.7, "Removing the previous system");
        delete_subvolume_tree(top, "@system")?;
    }
    for name in ["@system", "@home", "@snapshots"] {
        if !top.join(name).exists() {
            run("btrfs", &["subvolume", "create", top.join(name).to_str().unwrap()])?;
        }
    }
    Ok(())
}

fn mount_target(layout: &Layout, mounts: &mut Mounts) -> Result<PathBuf> {
    let t = PathBuf::from(TARGET);
    mounts.mount(&layout.btrfs, &t, &["-o", &format!("subvol=@system,{BTRFS_OPTS}")])?;
    mounts.mount(&layout.btrfs, &t.join("home"), &["-o", &format!("subvol=@home,{BTRFS_OPTS}")])?;
    mounts.mount(&layout.btrfs, &t.join(".snapshots"), &["-o", &format!("subvol=@snapshots,{BTRFS_OPTS}")])?;
    if let Some(b) = &layout.boot {
        mounts.mount(b, &t.join("boot"), &[])?;
    }
    mounts.mount(&layout.esp, &t.join("boot/efi"), &[])?;
    Ok(t)
}

fn copy_system(target: &Path) -> Result<()> {
    let mut args: Vec<String> = ["-aAXH", "--numeric-ids", "--one-file-system", "--info=progress2", "--no-inc-recursive"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    args.extend(EXCLUDES.iter().map(|e| format!("--exclude={e}")));
    args.push("/".into());
    args.push(format!("{}/", target.display()));
    info(format!("$ rsync {}", args.join(" ")));
    let mut child = Command::new("rsync")
        .args(&args)
        .env("LC_ALL", "C.UTF-8")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("cannot start rsync")?;
    let mut stderr = child.stderr.take().unwrap();
    let err_thread = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr.read_to_string(&mut s);
        s
    });
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let mut buf = vec![];
    let mut last = Instant::now() - Duration::from_secs(1);
    // rsync rewrites its progress line with '\r'.
    while reader.read_until(b'\r', &mut buf)? > 0 {
        let line = String::from_utf8_lossy(&buf).to_string();
        buf.clear();
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if let Some(pct) = tokens.iter().find_map(|t| t.strip_suffix('%').and_then(|n| n.parse::<f64>().ok())) {
            if last.elapsed() >= Duration::from_millis(400) {
                last = Instant::now();
                let speed = tokens.iter().find(|t| t.ends_with("/s")).copied().unwrap_or("");
                progress(pct / 100.0, &format!("{pct:.0}% copied  ·  {speed}"));
            }
        }
    }
    let status = child.wait()?;
    let err = err_thread.join().unwrap_or_default();
    for l in err.lines().take(40) {
        log(&format!("  {l}"));
    }
    // 24: some files vanished during the copy (caches of the live session).
    match status.code() {
        Some(0) | Some(24) => Ok(()),
        _ => bail!("Copying the system failed ({status}): {}", err.lines().last().unwrap_or("")),
    }
}

fn set_kv(text: &str, key: &str, value: &str) -> String {
    let line = format!("{key}={value}");
    let mut found = false;
    let mut out: Vec<String> = text
        .lines()
        .map(|l| {
            let t = l.trim_start_matches('#').trim_start();
            if t.starts_with(&format!("{key}=")) && !found {
                found = true;
                line.clone()
            } else {
                l.to_string()
            }
        })
        .collect();
    if !found {
        out.push(line);
    }
    out.join("\n") + "\n"
}

/// Edit gdm's daemon.conf: drop live-session settings, add ours under [daemon].
pub fn gdm_config(text: &str, autologin: Option<&str>, session: Option<&str>) -> String {
    let mut out = vec![];
    for l in text.lines() {
        let t = l.trim();
        if ["AutomaticLoginEnable=", "AutomaticLogin=", "InitialSetupEnable=", "TimedLogin", "DefaultSession="].iter().any(|k| t.starts_with(k)) {
            continue;
        }
        out.push(l.to_string());
        if t == "[daemon]" {
            out.push("InitialSetupEnable=false".into());
            if let Some(sess) = session {
                out.push(format!("DefaultSession={sess}.desktop"));
            }
            if let Some(u) = autologin {
                out.push("AutomaticLoginEnable=true".into());
                out.push(format!("AutomaticLogin={u}"));
            }
        }
    }
    if !out.iter().any(|l| l.trim() == "[daemon]") {
        out.insert(0, "[daemon]".into());
        out.insert(1, "InitialSetupEnable=false".into());
        if let Some(u) = autologin {
            out.insert(2, format!("AutomaticLogin={u}"));
            out.insert(2, "AutomaticLoginEnable=true".into());
        }
    }
    out.join("\n") + "\n"
}

const USER_GROUPS: [&str; 12] = ["sudo", "adm", "audio", "video", "plugdev", "netdev", "lpadmin", "users", "input", "render", "bluetooth", "scanner"];

/// Create the administrator account inside `root` ("/" for the running system).
pub fn create_user(root: &Path, u: &User) -> Result<()> {
    let r = root.to_str().unwrap();
    let groups: Vec<&str> = USER_GROUPS.into_iter().filter(|g| query("chroot", &[r, "getent", "group", g]).is_some()).collect();
    let groups = groups.join(",");
    let gecos: String = u.full_name.chars().filter(|c| !":,\n".contains(*c)).collect();
    let home = root.join("home").join(&u.username);
    let home_exists = home.is_dir();
    let mut args = vec!["useradd", "-s", "/bin/bash", "-c", gecos.as_str(), "-G", groups.as_str()];
    args.push(if home_exists { "-M" } else { "-m" });
    args.push(&u.username);
    chroot(root, &args)?;
    chroot_input(root, &["chpasswd"], format!("{}:{}\n", u.username, u.password).as_bytes())?;
    if home_exists {
        info(format!("Keeping existing home folder /home/{}", u.username));
        chroot(root, &["chown", "-R", &format!("{0}:{0}", u.username), &format!("/home/{}", u.username)])?;
    }
    let done_marker = home.join(".config/gnome-initial-setup-done");
    if !done_marker.exists() {
        write_file(&done_marker, "yes", 0o644)?;
        chroot(root, &["chown", "-R", &format!("{0}:{0}", u.username), &format!("/home/{}/.config", u.username)])?;
    }
    Ok(())
}

pub fn set_root_password(root: &Path, password: Option<&str>) -> Result<()> {
    match password {
        Some(p) if !p.is_empty() => {
            chroot_input(root, &["chpasswd"], format!("root:{p}\n").as_bytes())?;
        }
        _ => {
            // Root login stays locked; full root access through sudo. The
            // emergency and rescue shells must still open without a root
            // password, or a broken boot would leave no way to repair it.
            chroot(root, &["usermod", "-p", "*", "root"])?;
            for unit in ["emergency", "rescue"] {
                write_file(
                    root.join(format!("etc/systemd/system/{unit}.service.d/man-sulogin.conf")),
                    "[Service]\n# MAN: root is locked (sudo is used instead); allow the repair shell.\nEnvironment=SYSTEMD_SULOGIN_FORCE=1\n",
                    0o644,
                )?;
            }
        }
    }
    Ok(())
}

/// First start: GDM logs in the locked OOBE account into the MAN Setup session.
fn setup_oobe(t: &Path) -> Result<()> {
    let r = t.to_str().unwrap();
    if query("chroot", &[r, "id", "-u", OOBE_USER]).is_none() {
        chroot(t, &["useradd", "-r", "-m", "-d", "/var/lib/man-oobe", "-s", "/bin/bash", "-c", "MAN Setup", OOBE_USER])?;
    }
    chroot(t, &["usermod", "-p", "!", OOBE_USER])?;
    let groups: Vec<&str> = ["audio", "video", "input", "render", "netdev"].into_iter().filter(|g| query("chroot", &[r, "getent", "group", g]).is_some()).collect();
    chroot(t, &["usermod", "-a", "-G", &groups.join(","), OOBE_USER])?;
    write_file(t.join("var/lib/AccountsService/users").join(OOBE_USER), "[User]\nSession=man-oobe\nSystemAccount=true\n", 0o600)?;
    let gdm = t.join("etc/gdm3/daemon.conf");
    let text = read_to_string(&gdm).unwrap_or_default();
    write_file(&gdm, &gdm_config(&text, Some(OOBE_USER), Some("man-oobe")), 0o644)?;
    write_file(t.join(OOBE_PENDING.trim_start_matches('/')), "", 0o644)?;
    Ok(())
}

fn bind_system(target: &Path, mounts: &mut Mounts) -> Result<()> {
    mounts.mount("/dev", &target.join("dev"), &["--bind"])?;
    mounts.mount("devpts", &target.join("dev/pts"), &["-t", "devpts"])?;
    mounts.mount("proc", &target.join("proc"), &["-t", "proc"])?;
    mounts.mount("sysfs", &target.join("sys"), &["-t", "sysfs"])?;
    if Path::new("/sys/firmware/efi/efivars").is_dir() {
        mounts.mount("efivarfs", &target.join("sys/firmware/efi/efivars"), &["-t", "efivarfs"])?;
    }
    mounts.mount("tmpfs", &target.join("run"), &["-t", "tmpfs"])?;
    if Path::new("/run/udev").is_dir() {
        mounts.mount("/run/udev", &target.join("run/udev"), &["--bind"])?;
    }
    Ok(())
}

fn configure(plan: &Plan, layout: &Layout, t: &Path) -> Result<()> {
    let btrfs_uuid = uuid_of(&layout.btrfs)?;
    let esp_uuid = uuid_of(&layout.esp)?;
    progress(0.05, "Writing file system table");
    let mut fstab = String::from("# MAN file systems, written by the installer.\n");
    for (mp, sub) in [("/", "@system"), ("/home", "@home"), ("/.snapshots", "@snapshots")] {
        fstab += &format!("UUID={btrfs_uuid}  {mp:<12} btrfs  subvol={sub},{BTRFS_OPTS}  0 0\n");
    }
    if let Some(b) = &layout.boot {
        fstab += &format!("UUID={}  /boot        ext4   defaults,noatime,nofail,x-systemd.device-timeout=10s  0 2\n", uuid_of(b)?);
    }
    // nofail: a missing boot or EFI partition must not keep the desktop
    // from starting (for example after restoring an older snapshot).
    fstab += &format!("UUID={esp_uuid}  /boot/efi    vfat   umask=0077,shortname=winnt,nofail,x-systemd.device-timeout=10s  0 2\n");
    write_file(t.join("etc/fstab"), &fstab, 0o644)?;
    if plan.encrypt || layout.btrfs.starts_with("/dev/mapper/") {
        let luks_uuid = uuid_of(&layout.root_part)?;
        write_file(t.join("etc/crypttab"), &format!("# <name> <device> <key> <options>\n{MAPPER} UUID={luks_uuid} none luks,discard\n"), 0o600)?;
        let hook = t.join("etc/cryptsetup-initramfs/conf-hook");
        let text = read_to_string(&hook).unwrap_or_default();
        write_file(&hook, &set_kv(&text, "CRYPTSETUP", "y"), 0o644)?;
    } else {
        let _ = std::fs::remove_file(t.join("etc/crypttab"));
    }

    progress(0.15, "Setting computer name and identity");
    write_file(t.join("etc/hostname"), &format!("{}\n", plan.hostname), 0o644)?;
    write_file(
        t.join("etc/hosts"),
        &format!("127.0.0.1 localhost\n127.0.1.1 {}\n\n::1 localhost ip6-localhost ip6-loopback\nff02::1 ip6-allnodes\nff02::2 ip6-allrouters\n", plan.hostname),
        0o644,
    )?;
    // Empty machine-id: systemd generates a unique one on first boot.
    write_file(t.join("etc/machine-id"), "", 0o444)?;
    let dbus_id = t.join("var/lib/dbus/machine-id");
    let _ = std::fs::remove_file(&dbus_id);
    let _ = std::os::unix::fs::symlink("/etc/machine-id", &dbus_id);
    std::fs::create_dir_all(t.join("root"))?;
    run("chmod", &["0700", t.join("root").to_str().unwrap()])?;

    progress(0.25, "Removing the live session accounts");
    let _ = chroot(t, &["userdel", "-f", "MANInstaller"]);
    let _ = std::fs::remove_file(t.join("etc/sudoers.d/90-maninstaller"));
    let _ = std::fs::remove_dir_all(t.join("home/MANInstaller"));
    // The image's development account sits on Debian's `man` system user
    // (used by man-db). Return it to a locked system account.
    if query("chroot", &[t.to_str().unwrap(), "id", "-u", "man"]).as_deref() == Some("1000") {
        if query("chroot", &[t.to_str().unwrap(), "getent", "passwd", "6"]).is_none() {
            chroot(t, &["usermod", "-u", "6", "man"])?;
        }
        if query("chroot", &[t.to_str().unwrap(), "getent", "group", "12"]).is_none() {
            chroot(t, &["groupmod", "-g", "12", "man"])?;
        }
        chroot(t, &["usermod", "-d", "/var/cache/man", "-s", "/usr/sbin/nologin", "-c", "man", "-p", "*", "man"])?;
        let _ = chroot(t, &["chown", "-R", "man:man", "/var/cache/man"]);
    }

    // man-live-config declares the live session's `man` account (UID 1000,
    // /home/man) for systemd-sysusers/tmpfiles. Mask both on installed
    // systems so they can never claim the user's UID or create /home/man.
    for conf in ["etc/sysusers.d/man.conf", "etc/tmpfiles.d/man.conf"] {
        let p = t.join(conf);
        std::fs::create_dir_all(p.parent().unwrap())?;
        let _ = std::fs::remove_file(&p);
        std::os::unix::fs::symlink("/dev/null", &p)?;
    }

    // New accounts get GNOME's default avatar, not the Debian logo from skel.
    for f in ["etc/skel/.face", "etc/skel/.face.icon"] {
        let _ = std::fs::remove_file(t.join(f));
    }
    match &plan.user {
        Some(u) => {
            progress(0.35, "Creating your account");
            create_user(t, u)?;
            set_root_password(t, plan.root_password.as_deref())?;
            let gdm = t.join("etc/gdm3/daemon.conf");
            let text = read_to_string(&gdm).unwrap_or_default();
            write_file(&gdm, &gdm_config(&text, u.autologin.then_some(u.username.as_str()), None), 0o644)?;
        }
        None => {
            progress(0.35, "Preparing MAN Setup for the first start");
            set_root_password(t, None)?;
            setup_oobe(t)?;
        }
    }

    progress(0.45, "Setting time zone and keyboard");
    let lt = t.join("etc/localtime");
    let _ = std::fs::remove_file(&lt);
    std::os::unix::fs::symlink(format!("/usr/share/zoneinfo/{}", plan.timezone), &lt)?;
    write_file(t.join("etc/timezone"), &format!("{}\n", plan.timezone), 0o644)?;
    let (layout_name, variant) = plan.keyboard.split_once('+').unwrap_or((&plan.keyboard, ""));
    write_file(
        t.join("etc/default/keyboard"),
        &format!("XKBMODEL=\"pc105\"\nXKBLAYOUT=\"{layout_name}\"\nXKBVARIANT=\"{variant}\"\nXKBOPTIONS=\"\"\nBACKSPACE=\"guess\"\n"),
        0o644,
    )?;
    write_file(
        t.join("etc/dconf/db/local.d/00-man-keyboard"),
        &format!("[org/gnome/desktop/input-sources]\nsources=[('xkb', '{}')]\n", plan.keyboard),
        0o644,
    )?;
    let _ = chroot(t, &["dconf", "update"]);
    // The live session's language unless the plan names another one; the
    // locale data was copied with the system.
    if let Some(lang) = plan.language.clone().or_else(|| crate::locale::current_language(Path::new("/"))) {
        if let Err(e) = crate::locale::set_system_language(t, &lang) {
            info(format!("Warning: language {lang} not set: {e:#}"));
        }
    }

    progress(0.55, "Removing the installer from the new system");
    let _ = chroot(t, &["dpkg", "--purge", "man-installer"]);
    let conf = t.join(SNAPSHOT_CONFIG.trim_start_matches('/'));
    if !conf.exists() {
        write_file(&conf, &SnapshotConfig::default().render(), 0o644)?;
    }

    progress(0.65, "Building the boot image (initramfs)");
    let mut kernels: Vec<String> = std::fs::read_dir(t.join("boot"))?
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_prefix("vmlinuz-")).map(String::from))
        .filter(|v| t.join("usr/lib/modules").join(v).is_dir())
        .collect();
    kernels.sort();
    if kernels.is_empty() {
        bail!("No kernel with matching modules was found in /boot.");
    }
    for v in &kernels {
        let mode = if t.join(format!("boot/initrd.img-{v}")).exists() { "-u" } else { "-c" };
        if let Err(e) = chroot(t, &["update-initramfs", mode, "-k", v]) {
            if v.starts_with("7.") {
                return Err(e);
            }
            info(format!("Warning: initramfs for {v} failed: {e:#}"));
        }
    }

    // Installed from a DUHACH! partition on this disk: MAN removes it on its
    // first start (man-duhach-cleanup) and grows into the space.
    if let Some(part) = probe::installer_partition() {
        if query("lsblk", &["-ndo", "PKNAME", &part]).map(|d| format!("/dev/{d}")).as_deref() == Some(plan.disk.as_str()) {
            if let Some(puuid) = query("blkid", &["-s", "PARTUUID", "-o", "value", &part]) {
                write_file(t.join("etc/man/duhach-installer"), &format!("{puuid}\n"), 0o644)?;
            }
        }
    }

    progress(0.85, "Configuring GRUB");
    let grub = t.join("etc/default/grub");
    let mut text = read_to_string(&grub).unwrap_or_default();
    for (k, v) in [
        ("GRUB_DISTRIBUTOR", "\"MAN\""),
        // With Windows on the computer the menu is a real choice: more time.
        ("GRUB_TIMEOUT", if plan.mode == Mode::Alongside { "5" } else { "3" }),
        ("GRUB_TIMEOUT_STYLE", "menu"),
        ("GRUB_DISABLE_OS_PROBER", "true"),
        ("GRUB_CMDLINE_LINUX_DEFAULT", "\"quiet\""),
    ] {
        text = set_kv(&text, k, v);
    }
    write_file(&grub, &text, 0o644)?;
    Ok(())
}

fn install_bootloader(t: &Path) -> Result<()> {
    progress(0.1, "Installing GRUB to the EFI system partition");
    let base = ["grub-install", "--target=x86_64-efi", "--efi-directory=/boot/efi", "--bootloader-id=MAN", "--no-uefi-secure-boot"];
    if let Err(e) = chroot(t, &base) {
        // Some firmware refuses new boot entries; the fallback path below
        // still boots, so this is a warning rather than a failure.
        info(format!("Warning: could not register a firmware boot entry ({e:#}); using the fallback boot path."));
        let mut no_nvram = base.to_vec();
        no_nvram.push("--no-nvram");
        chroot(t, &no_nvram)?;
    }
    // The fallback path (EFI/BOOT/BOOTX64.EFI) belongs to Windows when the
    // EFI partition is shared with it; MAN's own boot entry is enough then.
    if t.join("boot/efi/EFI/Microsoft").is_dir() {
        info("Windows uses this EFI partition: its fallback boot loader is kept.");
    } else {
        progress(0.5, "Installing the fallback boot loader");
        let mut removable = base.to_vec();
        removable.extend(["--removable", "--no-nvram"]);
        chroot(t, &removable)?;
    }
    // GRUB reads grub.cfg from inside @system, so it must exist before the
    // first snapshot is taken: restoring that snapshot has to stay bootable.
    progress(0.8, "Writing the boot menu");
    chroot(t, &["update-grub"])?;
    Ok(())
}

pub fn install(plan: Plan) -> Result<String> {
    step(0);
    let source = probe::source_bytes();
    let (disk, disks) = validate(&plan, source)?;
    info(format!("Installing to {} ({} {}), mode {:?}, encrypted: {}", disk.path, disk.model, human_size(disk.size), plan.mode, plan.encrypt));

    let mut mounts = Mounts::default();
    step(1);
    let layout = match plan.mode {
        Mode::Erase => partition_disk(&plan, &disk, &mut mounts)?,
        Mode::Manual => prepare_manual(&plan, &mut mounts)?,
        Mode::Reinstall => prepare_reinstall(&plan, &disks, &mut mounts)?,
        Mode::Alongside => prepare_alongside(&plan, &disk, &mut mounts)?,
    };
    create_subvolumes(&layout, plan.mode == Mode::Reinstall, &mut mounts)?;
    let t = mount_target(&layout, &mut mounts)?;
    progress(1.0, "Disk ready");

    step(2);
    copy_system(&t)?;

    step(3);
    bind_system(&t, &mut mounts)?;
    configure(&plan, &layout, &t)?;

    step(4);
    install_bootloader(&t)?;

    step(5);
    progress(0.2, "Creating a snapshot of the fresh system");
    let id = snapshot_id(now());
    let dir = Path::new(TOP).join("@snapshots").join(&id);
    std::fs::create_dir_all(&dir)?;
    run("btrfs", &["subvolume", "snapshot", "-r", &format!("{TOP}/@system"), dir.join("snapshot").to_str().unwrap()])?;
    write_info(&dir, &SnapshotInfo { id, created: rfc3339(now()), description: "Fresh installation".into(), kind: "install".into() })?;
    progress(0.6, "Adding the snapshot to the boot menu");
    chroot(&t, &["update-grub"])?;

    step(6);
    progress(0.5, "Unmounting");
    mounts.release();
    progress(1.0, "Done");
    Ok(format!("MAN was installed on {} ({}).", disk.model, disk.path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kv_replace_and_append() {
        let t = "A=1\n#GRUB_TIMEOUT=5\nB=2\n";
        let r = set_kv(t, "GRUB_TIMEOUT", "3");
        assert_eq!(r, "A=1\nGRUB_TIMEOUT=3\nB=2\n");
        assert!(set_kv("A=1\n", "X", "y").ends_with("X=y\n"));
    }

    #[test]
    fn gdm_autologin() {
        let t = "[daemon]\nAutomaticLoginEnable=true\nAutomaticLogin=MANInstaller\n[security]\n";
        let r = gdm_config(t, None, None);
        assert!(!r.contains("MANInstaller"));
        assert!(r.contains("InitialSetupEnable=false"));
        let r = gdm_config(t, Some("anna"), Some("man-oobe"));
        assert!(r.contains("AutomaticLogin=anna"));
        assert!(r.contains("DefaultSession=man-oobe.desktop"));
        assert_eq!(r.matches("AutomaticLoginEnable=true").count(), 1);
    }

    #[test]
    fn partitions() {
        assert_eq!(part_path("/dev/vdb", 2), "/dev/vdb2");
        assert_eq!(part_path("/dev/nvme0n1", 1), "/dev/nvme0n1p1");
    }
}
