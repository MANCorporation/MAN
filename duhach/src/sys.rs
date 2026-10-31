//! Everything DUHACH! does to the computer. On Windows the work is done by
//! PowerShell (storage cmdlets, bcdedit); elsewhere a demo backend simulates
//! a typical PC so the interface can be developed and tried out.
#![cfg_attr(not(windows), allow(dead_code))]
use serde::Deserialize;
use std::path::Path;

pub const GIB: u64 = 1 << 30;
const MIB: u64 = 1 << 20;
/// Space MAN needs at least (system, boot partition and room to work).
pub const MIN_MAN: u64 = 20 * GIB;
/// Free space Windows always keeps on top of what it cannot give away.
pub const WINDOWS_RESERVE: u64 = 10 * GIB;
/// Volume label of the installer partition; the MAN live system and the
/// installer recognise it.
pub const LIVE_LABEL: &str = "MAN_LIVE";
/// Name of the one-time firmware boot entry.
pub const BOOT_ENTRY: &str = "MAN Installer";
/// GRUB built for starting the MAN ISO's contents from a partition.
pub const EFI_LOADER: &str = r"\EFI\MAN-Installer\grubx64.efi";

/// The computer as DUHACH! sees it.
#[derive(Deserialize, Debug, Clone, Default)]
pub struct SysInfo {
    pub admin: bool,
    pub uefi: bool,
    pub secure_boot: bool,
    pub windows: String,
    pub disk_number: u32,
    pub disk_model: String,
    pub disk_size: u64,
    pub gpt: bool,
    pub c_partition: u32,
    pub c_offset: u64,
    pub c_size: u64,
    /// Smallest size Windows can shrink C: to.
    pub c_min: u64,
    pub c_free: u64,
    pub bitlocker: bool,
    pub on_battery: bool,
    /// The disks run behind Intel RST / AMD RAID (not VMD): Linux has no
    /// driver for that mode, so the MAN installer would see no disk.
    #[serde(default)]
    pub raid_mode: bool,
    /// Windows is set to start in Safe Mode once (DUHACH!'s AHCI switch).
    #[serde(default)]
    pub ahci_pending: bool,
    /// A MAN system (Linux partitions) is already on the disk.
    pub man_installed: bool,
    /// An installer partition made earlier by DUHACH!.
    pub prepared: Option<Prepared>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Prepared {
    pub partition: u32,
    pub size: u64,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub boot_entry: String,
}

/// A checked MAN ISO.
#[derive(Deserialize, Debug, Clone, Default)]
pub struct IsoInfo {
    #[serde(default)]
    pub path: String,
    pub id: String,
    pub bytes: u64,
    pub files: u64,
}

impl IsoInfo {
    /// "MAN-10.0-202610201930-5afa23" -> "10.0"
    pub fn version(&self) -> String {
        self.id.split('-').nth(1).unwrap_or("?").to_string()
    }
    /// Size of the installer partition: the ISO's files plus room, in 256 MiB steps.
    pub fn partition_bytes(&self) -> u64 {
        let want = self.bytes + self.bytes / 20 + 192 * MIB;
        want.div_ceil(256 * MIB) * 256 * MIB
    }
}

impl SysInfo {
    /// Most space C: can give to MAN (installer partition not included).
    pub fn max_man(&self, installer: u64) -> u64 {
        let keep = self.c_min.max(self.c_size - self.c_free) + WINDOWS_RESERVE;
        self.c_size.saturating_sub(keep).saturating_sub(installer) / GIB * GIB
    }
}

/// A problem found by the checks: blocking ones stop DUHACH!.
pub struct Check {
    pub title: String,
    pub detail: String,
    pub ok: bool,
    pub blocking: bool,
}

pub fn checks(s: &SysInfo, min_installer: u64) -> Vec<Check> {
    let mut v = vec![];
    let mut add = |ok: bool, blocking: bool, title: &str, good: &str, bad: &str| {
        v.push(Check { title: title.into(), detail: if ok { good.into() } else { bad.into() }, ok, blocking: blocking && !ok });
    };
    add(s.admin, true, "Administrator rights", "DUHACH! can change partitions and the boot menu.", "Start DUHACH! with “Run as administrator”.");
    add(s.uefi, true, "UEFI firmware", "Windows was started in UEFI mode, like MAN needs.", "This PC starts Windows in legacy BIOS mode. MAN needs UEFI.");
    add(s.gpt, true, "GPT disk", &format!("{} uses GPT partitions.", s.disk_model), "The Windows disk uses the old MBR partition style. MAN needs GPT.");
    add(
        !s.secure_boot,
        true,
        "Secure Boot",
        "Secure Boot is off, so MAN can start.",
        "Secure Boot is on. MAN cannot start until it is turned off in the UEFI settings.",
    );
    add(
        !s.raid_mode,
        true,
        "Storage mode",
        "The disk controller runs in AHCI or NVMe mode, which MAN can read.",
        if s.ahci_pending {
            "Windows is ready for AHCI. Restart into the UEFI settings and set the storage mode from RAID / Intel RST to AHCI."
        } else {
            "The disk controller runs in RAID / Intel RST mode, which MAN cannot read. Press “Switch to AHCI…”: DUHACH! prepares Windows so it keeps starting."
        },
    );
    let room = s.max_man(min_installer) >= MIN_MAN;
    add(
        room,
        true,
        "Free space on C:",
        &format!("{} free; up to {} can be given to MAN.", human(s.c_free), human(s.max_man(min_installer))),
        &format!("MAN needs at least {} and Windows must keep {} free. Free up space on C: first.", human(MIN_MAN), human(WINDOWS_RESERVE)),
    );
    if s.man_installed {
        v.push(Check {
            title: "MAN".into(),
            detail: "MAN is already installed on this PC. Preparing again installs a second copy next to it.".into(),
            ok: false,
            blocking: false,
        });
    }
    if s.bitlocker {
        v.push(Check {
            title: "BitLocker".into(),
            detail: "Drive C: is encrypted with BitLocker. Keep your recovery key at hand: Windows may ask for it after MAN is installed.".into(),
            ok: false,
            blocking: false,
        });
    }
    if s.on_battery {
        v.push(Check { title: "Power".into(), detail: "Connect the charger: a power loss while partitions change can damage data.".into(), ok: false, blocking: false });
    }
    v
}

/// Sizes the way Windows shows them (binary units, written GB and MB).
pub fn human(bytes: u64) -> String {
    let gb = bytes as f64 / GIB as f64;
    if gb >= 100.0 {
        format!("{gb:.0} GB")
    } else if gb >= 1.0 {
        format!("{gb:.1} GB")
    } else {
        format!("{:.0} MB", bytes as f64 / MIB as f64)
    }
}

/// What "Prepare MAN" does, decided on the Ready page.
#[derive(Debug, Clone)]
pub struct Plan {
    pub iso: IsoInfo,
    pub man_bytes: u64,
    pub installer_bytes: u64,
    pub sys: SysInfo,
}

impl Plan {
    pub fn new_c_size(&self) -> u64 {
        (self.sys.c_size - self.man_bytes - self.installer_bytes) / MIB * MIB
    }
    /// The installer partition sits at the end of the freed space, so MAN's
    /// partitions come right after Windows and can later grow into it.
    pub fn installer_offset(&self) -> u64 {
        let end = self.sys.c_offset + self.sys.c_size;
        (end - self.installer_bytes) / MIB * MIB
    }
}

pub enum Progress {
    Step(usize),
    Detail(f64, String),
}

/// Steps shown while preparing (index used by [`Progress::Step`]).
pub const STEPS: [&str; 5] = [
    "Making room: shrinking Windows (C:)",
    "Creating the MAN installer partition",
    "Copying MAN",
    "Adding the MAN installer to the boot menu",
    "Final checks",
];

// ───────────────────────────── Windows ─────────────────────────────

#[cfg(windows)]
mod imp {
    use super::*;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    /// Quote for a single-quoted PowerShell string.
    fn q(s: &str) -> String {
        format!("'{}'", s.replace('\'', "''"))
    }

    fn ps(script: &str) -> Result<String, String> {
        let full = format!(
            "$ErrorActionPreference = 'Stop'\n$ProgressPreference = 'SilentlyContinue'\n[Console]::OutputEncoding = [Text.Encoding]::UTF8\ntry {{\n{script}\n}} catch {{ [Console]::Error.WriteLine($_.Exception.Message); exit 1 }}\n"
        );
        // -EncodedCommand: the whole script as one block (UTF-16LE, base64);
        // a script on stdin would run line by line.
        let utf16: Vec<u8> = full.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        let out = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand", &base64(&utf16)])
            .creation_flags(CREATE_NO_WINDOW)
            .stdin(Stdio::null())
            .output()
            .map_err(|e| format!("Cannot start PowerShell: {e}"))?;
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if out.status.success() {
            Ok(stdout)
        } else {
            let err = String::from_utf8_lossy(&out.stderr);
            let msg = err.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with("#< CLIXML")).next_back().unwrap_or("PowerShell failed").to_string();
            Err(msg)
        }
    }

    fn base64(data: &[u8]) -> String {
        const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut s = String::with_capacity(data.len().div_ceil(3) * 4);
        for c in data.chunks(3) {
            let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
            for i in 0..4 {
                s.push(if i <= c.len() { T[(n >> (18 - 6 * i) & 63) as usize] as char } else { '=' });
            }
        }
        s
    }

    fn json<T: serde::de::DeserializeOwned>(script: &str) -> Result<T, String> {
        let out = ps(script)?;
        let line = out.lines().rev().find(|l| l.starts_with('{')).unwrap_or("");
        serde_json::from_str(line).map_err(|e| format!("Unexpected answer from Windows ({e}): {out}"))
    }

    const STATE: &str = r"$state = Join-Path $env:ProgramData 'DUHACH!\state.json'";
    const CLEANUP_PS1: &str = include_str!("../data/windows/cleanup.ps1");
    /// Removes DUHACH!'s loader from the EFI system partition (see step 3).
    const ESP_REMOVE: &str = r#"$l = [char[]](90..68) | ? { -not (Test-Path "$($_):\") } | select -First 1
mountvol "$($l):" /S 2>&1 | Out-Null
if ($LASTEXITCODE -eq 0) { Remove-Item -Recurse -Force -LiteralPath "$($l):\EFI\MAN-Installer" -EA SilentlyContinue; mountvol "$($l):" /D | Out-Null }"#;

    pub fn system_info() -> Result<SysInfo, String> {
        json(&format!(
            r#"{STATE}
$r = [ordered]@{{}}
$r.admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
$r.uefi = ($env:firmware_type -eq 'UEFI')
$r.secure_boot = $false
try {{ $r.secure_boot = [bool](Confirm-SecureBootUEFI) }} catch {{}}
$r.windows = [string](Get-CimInstance Win32_OperatingSystem).Caption
$c = Get-Partition -DriveLetter C
$d = Get-Disk -Number $c.DiskNumber
$r.disk_number = [int]$d.Number
$r.disk_model = [string]$d.FriendlyName
$r.disk_size = [uint64]$d.Size
$r.gpt = ([string]$d.PartitionStyle -eq 'GPT')
$r.c_partition = [int]$c.PartitionNumber
$r.c_offset = [uint64]$c.Offset
$r.c_size = [uint64]$c.Size
$r.c_min = [uint64](Get-PartitionSupportedSize -DiskNumber $c.DiskNumber -PartitionNumber $c.PartitionNumber).SizeMin
$r.c_free = [uint64](Get-Volume -DriveLetter C).SizeRemaining
$r.bitlocker = $false
try {{ $r.bitlocker = ([string](Get-BitLockerVolume -MountPoint 'C:').ProtectionStatus -eq 'On') }} catch {{}}
$r.on_battery = $false
try {{ $b = @(Get-CimInstance Win32_Battery); if ($b.Count -gt 0 -and $b[0].BatteryStatus -ne 2) {{ $r.on_battery = $true }} }} catch {{}}
# Intel RST / AMD RAID controllers (PCI class 0104) other than Intel VMD,
# which the MAN kernel supports.
$vmd = @('201D','28C0','467F','4C3D','7D0B','9A0B','A77F','AD0B','B60B','B06F')
$r.raid_mode = $false
foreach ($dev in @(Get-PnpDevice -PresentOnly -Class SCSIAdapter,HDC -ErrorAction SilentlyContinue)) {{
    $ids = (@($dev.HardwareID) -join ' ').ToUpper()
    if ($ids -notmatch 'CC_0104') {{ continue }}
    if ($ids -match 'VEN_8086&DEV_([0-9A-F]{{4}})') {{ if ($vmd -notcontains $Matches[1]) {{ $r.raid_mode = $true }} }}
    elseif ($ids -match 'VEN_1022&') {{ $r.raid_mode = $true }}
}}
$r.ahci_pending = [bool]((bcdedit /enum '{{current}}' | Out-String) -match 'safeboot')
$linux = @('{{4f68bce3-e8cd-4db1-96e7-fbcaf984b709}}', '{{ca7d7ccb-63ed-4c53-861c-1742536059cc}}', '{{0fc63daf-8483-4772-8e79-3d69d8477de4}}')
$parts = @(Get-Partition -DiskNumber $c.DiskNumber)
$r.man_installed = [bool](@($parts | Where-Object {{ $linux -contains $_.GptType }}).Count)
$r.prepared = $null
foreach ($p in $parts) {{
    $v = $p | Get-Volume -ErrorAction SilentlyContinue
    if ($v -and $v.FileSystemLabel -eq '{LIVE_LABEL}') {{
        $info = [ordered]@{{ partition = [int]$p.PartitionNumber; size = [uint64]$p.Size; version = ''; boot_entry = '' }}
        if (Test-Path -LiteralPath $state) {{
            $s = Get-Content -LiteralPath $state -Raw | ConvertFrom-Json
            $info.version = [string]$s.version
            $info.boot_entry = [string]$s.boot_entry
        }}
        $r.prepared = $info
    }}
}}
# MAN was installed and removed the installer partition itself: drop the
# boot entry DUHACH! made and its notes.
if (-not $r.prepared -and (Test-Path -LiteralPath $state)) {{
    $s = Get-Content -LiteralPath $state -Raw | ConvertFrom-Json
    if ($s.boot_entry) {{ bcdedit /delete $s.boot_entry 2>&1 | Out-Null }}
    {esp_remove}
    Remove-Item -LiteralPath $state -ErrorAction SilentlyContinue
}}
$r | ConvertTo-Json -Compress -Depth 4"#,
            esp_remove = ESP_REMOVE
        ))
    }

    /// Mount the ISO (or reuse it if the user already opened it); returns
    /// the drive root and whether DUHACH! mounted it.
    fn mount_iso(path: &str) -> Result<(String, bool), String> {
        #[derive(Deserialize)]
        struct M {
            letter: String,
            mounted: bool,
        }
        let m: M = json(&format!(
            r#"$iso = {p}
$img = Get-DiskImage -ImagePath $iso
$mounted = $false
if (-not $img.Attached) {{ $img = Mount-DiskImage -ImagePath $iso -Access ReadOnly -StorageType ISO -PassThru; $mounted = $true }}
$letter = ''
for ($i = 0; $i -lt 20 -and -not $letter; $i++) {{ $letter = [string]($img | Get-Volume).DriveLetter; if (-not $letter) {{ Start-Sleep -Milliseconds 300 }} }}
if (-not $letter) {{ throw 'The ISO file was opened but Windows gave it no drive letter.' }}
[ordered]@{{ letter = $letter; mounted = $mounted }} | ConvertTo-Json -Compress"#,
            p = q(path)
        ))?;
        Ok((format!("{}:\\", m.letter), m.mounted))
    }

    fn unmount_iso(path: &str) {
        let _ = ps(&format!("Dismount-DiskImage -ImagePath {} | Out-Null", q(path)));
    }

    pub fn inspect_iso(path: &str) -> Result<IsoInfo, String> {
        let (root, mounted) = mount_iso(path)?;
        let r = super::inspect_tree(Path::new(&root));
        if mounted {
            unmount_iso(path);
        }
        r.map(|mut i| {
            i.path = path.to_string();
            i
        })
    }

    pub fn prepare(plan: &Plan, report: &dyn Fn(Progress)) -> Result<(), String> {
        let s = &plan.sys;
        let mut shrunk = false;
        let mut created: Option<u32> = None;
        let result = (|| -> Result<(), String> {
            report(Progress::Step(0));
            report(Progress::Detail(0.0, format!("C: {} → {}", human(s.c_size), human(plan.new_c_size()))));
            ps(&format!("Resize-Partition -DiskNumber {} -PartitionNumber {} -Size {}", s.disk_number, s.c_partition, plan.new_c_size()))?;
            shrunk = true;

            report(Progress::Step(1));
            #[derive(Deserialize)]
            struct P {
                partition: u32,
                letter: String,
            }
            let p: P = json(&format!(
                r#"$p = New-Partition -DiskNumber {disk} -Offset {off} -Size {size} -GptType '{{ebd0a0a2-b9e5-4433-87c0-68b6b72699c7}}'
$n = [int]$p.PartitionNumber
Format-Volume -Partition $p -FileSystem FAT32 -NewFileSystemLabel '{LIVE_LABEL}' -Confirm:$false -Force | Out-Null
Add-PartitionAccessPath -DiskNumber {disk} -PartitionNumber $n -AssignDriveLetter
$p = Get-Partition -DiskNumber {disk} -PartitionNumber $n
[ordered]@{{ partition = $n; letter = [string]$p.DriveLetter }} | ConvertTo-Json -Compress"#,
                disk = s.disk_number,
                off = plan.installer_offset(),
                size = plan.installer_bytes
            ))?;
            created = Some(p.partition);
            if p.letter.is_empty() {
                return Err("Windows gave the new partition no drive letter.".into());
            }
            let target = format!("{}:\\", p.letter);

            report(Progress::Step(2));
            let (root, mounted) = mount_iso(&plan.iso.path)?;
            let copied = super::copy_tree(Path::new(&root), Path::new(&target), plan.iso.bytes, report)
                .and_then(|_| super::verify_copy(Path::new(&root), Path::new(&target)));
            if mounted {
                unmount_iso(&plan.iso.path);
            }
            copied?;

            report(Progress::Step(3));
            let entry = ps(&format!(
                r#"{STATE}
if (Test-Path -LiteralPath $state) {{
    $old = Get-Content -LiteralPath $state -Raw | ConvertFrom-Json
    if ($old.boot_entry) {{ bcdedit /delete $old.boot_entry 2>&1 | Out-Null }}
}}
$out = (bcdedit /copy '{{bootmgr}}' /d '{BOOT_ENTRY}' 2>&1 | Out-String)
if ($out -notmatch '\{{[0-9a-fA-F-]{{36}}\}}') {{ throw "bcdedit could not create the boot entry: $out" }}
$g = $Matches[0]
# The loader goes onto the EFI system partition: Windows writes the firmware
# entry with its own partition number, which for a partition made in this
# session can differ from the GPT slot the firmware uses (then the firmware
# says "Not Found"). The ESP's number never changes; the loader finds the
# installer partition by its live/man-id file.
$esp = [char[]](90..68) | ? {{ -not (Test-Path "$($_):\") }} | select -First 1
mountvol "$($esp):" /S | Out-Null
if ($LASTEXITCODE -ne 0) {{ bcdedit /delete $g 2>&1 | Out-Null; throw 'The EFI system partition could not be opened.' }}
try {{
    New-Item -ItemType Directory -Force -Path "$($esp):\EFI\MAN-Installer" | Out-Null
    Copy-Item -Force -LiteralPath '{letter}:{EFI_LOADER}' -Destination "$($esp):{EFI_LOADER}"
}} catch {{ mountvol "$($esp):" /D | Out-Null; bcdedit /delete $g 2>&1 | Out-Null; throw }}
$steps = @(@('/set', $g, 'device', "partition=$($esp):"), @('/set', $g, 'path', '{EFI_LOADER}'),
           @('/set', '{{fwbootmgr}}', 'displayorder', $g, '/addlast'), @('/set', '{{fwbootmgr}}', 'bootsequence', $g))
foreach ($a in $steps) {{
    $o = (& bcdedit @a 2>&1 | Out-String)
    if ($LASTEXITCODE -ne 0) {{ mountvol "$($esp):" /D | Out-Null; bcdedit /delete $g 2>&1 | Out-Null; throw "bcdedit $($a -join ' '): $o" }}
}}
mountvol "$($esp):" /D | Out-Null
New-Item -ItemType Directory -Force -Path (Split-Path $state) | Out-Null
[ordered]@{{ boot_entry = $g; version = {ver}; partition = {part}; created = (Get-Date).ToString('o') }} | ConvertTo-Json | Set-Content -LiteralPath $state -Encoding UTF8
# Windows restores BCD boot entries into the firmware menu at every start:
# a startup task removes this one once MAN has deleted the installer partition.
$cleanup = Join-Path (Split-Path $state) 'cleanup.ps1'
Set-Content -LiteralPath $cleanup -Value {cleanup} -Encoding UTF8
$a = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument "-NoProfile -NonInteractive -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$cleanup`""
Register-ScheduledTask -TaskName 'DUHACH! cleanup' -Action $a -Trigger (New-ScheduledTaskTrigger -AtStartup) -User 'SYSTEM' -RunLevel Highest -Force | Out-Null
$g"#,
                letter = p.letter,
                cleanup = q(CLEANUP_PS1),
                ver = q(&plan.iso.version()),
                part = p.partition
            ))?;

            report(Progress::Step(4));
            if s.bitlocker {
                report(Progress::Detail(0.3, "Pausing BitLocker for the next restarts".into()));
                ps("Suspend-BitLocker -MountPoint 'C:' -RebootCount 3 | Out-Null")?;
            }
            // The partition stays out of Explorer; the boot entry already knows it.
            let _ = ps(&format!(
                "Remove-PartitionAccessPath -DiskNumber {} -PartitionNumber {} -AccessPath '{}:\\'",
                s.disk_number, p.partition, p.letter
            ));
            report(Progress::Detail(1.0, format!("Boot entry {}", entry.lines().last().unwrap_or(""))));
            Ok(())
        })();
        if result.is_err() {
            // Put Windows back the way it was, as far as possible.
            let _ = ps(ESP_REMOVE);
            if let Some(n) = created {
                let _ = ps(&format!("Remove-Partition -DiskNumber {} -PartitionNumber {n} -Confirm:$false", s.disk_number));
            }
            if shrunk {
                let _ = ps(&format!(
                    "Resize-Partition -DiskNumber {d} -PartitionNumber {p} -Size (Get-PartitionSupportedSize -DiskNumber {d} -PartitionNumber {p}).SizeMax",
                    d = s.disk_number,
                    p = s.c_partition
                ));
            }
        }
        result
    }

    /// Remove the installer partition and boot entry; give the space back
    /// to C: when MAN was not installed.
    pub fn remove(s: &SysInfo) -> Result<(), String> {
        let prep = s.prepared.clone().ok_or("No MAN installer partition was found.")?;
        ps(&format!(
            r#"{STATE}
if ('{entry}') {{ bcdedit /delete '{entry}' 2>&1 | Out-Null }}
{esp_remove}
Remove-Partition -DiskNumber {d} -PartitionNumber {n} -Confirm:$false
if (-not ${installed}) {{
    $max = (Get-PartitionSupportedSize -DiskNumber {d} -PartitionNumber {c}).SizeMax
    Resize-Partition -DiskNumber {d} -PartitionNumber {c} -Size $max
}}
Remove-Item -LiteralPath $state -ErrorAction SilentlyContinue
Unregister-ScheduledTask -TaskName 'DUHACH! cleanup' -Confirm:$false -ErrorAction SilentlyContinue"#,
            entry = prep.boot_entry.replace('\'', ""),
            esp_remove = ESP_REMOVE,
            d = s.disk_number,
            n = prep.partition,
            c = s.c_partition,
            installed = s.man_installed
        ))
        .map(|_| ())
    }

    /// Prepare Windows for the RAID → AHCI switch: next start in Safe Mode
    /// (loads the generic AHCI driver), which then clears itself and restarts
    /// normally. Then restart into the UEFI settings.
    pub fn switch_to_ahci(bitlocker: bool) -> Result<(), String> {
        ps(&format!(
            r#"{bl}
bcdedit /set '{{current}}' safeboot minimal | Out-Null
if ($LASTEXITCODE -ne 0) {{ throw 'bcdedit could not set Safe Mode for the next start.' }}
$run = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\RunOnce'
# "*": runs in Safe Mode too.
Set-ItemProperty -Path $run -Name '*DUHACH! AHCI' -Value 'cmd.exe /c bcdedit /deletevalue {{current}} safeboot && shutdown /r /t 3'"#,
            bl = if bitlocker { "Suspend-BitLocker -MountPoint 'C:' -RebootCount 3 | Out-Null" } else { "" }
        ))?;
        restart(true)
    }

    /// Restart now (the boot entry is used once), or into the UEFI settings.
    /// `/f` closes running programs; without it Windows can refuse with
    /// error 203 even though the firmware request was registered.
    pub fn restart(firmware: bool) -> Result<(), String> {
        let run = |args: &[&str]| {
            Command::new("shutdown.exe").creation_flags(CREATE_NO_WINDOW).args(args).status().map(|s| s.success()).unwrap_or(false)
        };
        if firmware {
            if run(&["/r", "/fw", "/f", "/t", "0"]) {
                return Ok(());
            }
            // No "restart to UEFI settings" on this PC: a plain restart; the
            // dialogs tell which key opens the settings.
        }
        if run(&["/r", "/f", "/t", "0"]) {
            Ok(())
        } else {
            Err("Windows refused to restart.".into())
        }
    }

    /// Start a reboot-to-firmware is only possible on UEFI PCs.
    pub fn demo() -> bool {
        false
    }
}

// ───────────────────────────── Demo ────────────────────────────────

#[cfg(not(windows))]
mod imp {
    use super::*;
    use std::time::Duration;

    fn pause(ms: u64) {
        std::thread::sleep(Duration::from_millis(ms));
    }

    pub fn system_info() -> Result<SysInfo, String> {
        pause(900);
        let prepared = std::env::var("DUHACH_DEMO_PREPARED").is_ok().then(|| Prepared { partition: 5, size: 2 * GIB, version: "10.0".into(), boot_entry: "{demo}".into() });
        Ok(SysInfo {
            admin: true,
            uefi: true,
            secure_boot: std::env::var("DUHACH_DEMO_SECUREBOOT").is_ok(),
            windows: "Microsoft Windows 11 Pro".into(),
            disk_number: 0,
            disk_model: "Samsung SSD 980 PRO 1TB".into(),
            disk_size: 1000 * GIB,
            gpt: true,
            c_partition: 3,
            c_offset: 116 * (1 << 20),
            c_size: 930 * GIB,
            c_min: 210 * GIB,
            c_free: 690 * GIB,
            bitlocker: std::env::var("DUHACH_DEMO_BITLOCKER").is_ok(),
            on_battery: false,
            man_installed: false,
            raid_mode: std::env::var("DUHACH_DEMO_RAID").is_ok(),
            ahci_pending: false,
            prepared,
        })
    }

    pub fn inspect_iso(path: &str) -> Result<IsoInfo, String> {
        pause(700);
        // A real ISO can be checked by pointing at an unpacked copy.
        let dir = Path::new(path).with_extension("");
        let mut info = if dir.is_dir() {
            super::inspect_tree(&dir)?
        } else if path.to_lowercase().ends_with(".iso") && Path::new(path).file_name().map(|n| n.to_string_lossy().starts_with("MAN")).unwrap_or(false) {
            IsoInfo { id: "MAN-10.0-202610201930-5afa23".into(), bytes: 1_330_000_000, files: 612, ..Default::default() }
        } else {
            return Err("This is not a MAN ISO: it has no MAN live system.".into());
        };
        info.path = path.into();
        Ok(info)
    }

    pub fn prepare(plan: &Plan, report: &dyn Fn(Progress)) -> Result<(), String> {
        for step in 0..STEPS.len() {
            report(Progress::Step(step));
            let n = if step == 2 { 60 } else { 8 };
            for i in 0..=n {
                let f = i as f64 / n as f64;
                let detail = if step == 2 {
                    format!("{} of {}  ·  180 MB/s", human((plan.iso.bytes as f64 * f) as u64), human(plan.iso.bytes))
                } else {
                    String::new()
                };
                report(Progress::Detail(f, detail));
                pause(60);
            }
            if std::env::var("DUHACH_DEMO_FAIL").ok().and_then(|v| v.parse::<usize>().ok()) == Some(step) {
                return Err("bcdedit could not create the boot entry: The parameter is incorrect.".into());
            }
        }
        Ok(())
    }

    pub fn remove(_s: &SysInfo) -> Result<(), String> {
        pause(1500);
        Ok(())
    }

    pub fn switch_to_ahci(_bitlocker: bool) -> Result<(), String> {
        pause(800);
        Err("This is the demo mode of DUHACH!: Windows was not changed.".into())
    }

    pub fn restart(_firmware: bool) -> Result<(), String> {
        Err("This is the demo mode of DUHACH!: nothing is restarted.".into())
    }

    pub fn demo() -> bool {
        true
    }
}

pub use imp::*;

/// Check an unpacked or mounted MAN ISO and count its files.
pub fn inspect_tree(root: &Path) -> Result<IsoInfo, String> {
    let need = ["live/man-id", "live/filesystem.squashfs", "live/vmlinuz", "live/initrd.img", "EFI/MAN-Installer/grubx64.efi"];
    if !root.join(need[0]).is_file() || !root.join(need[1]).is_file() {
        return Err("This is not a MAN ISO: it has no MAN live system.".into());
    }
    if let Some(m) = need.iter().find(|n| !root.join(n).is_file()) {
        return Err(format!("This MAN ISO is too old or damaged for DUHACH! ({m} is missing). Download a newer one."));
    }
    // A damaged copy of the ISO (interrupted download or copy) shows up here
    // first: man-id is short ASCII text like "MAN-10.2-202610221858-263f1b".
    const DAMAGED: &str = "This MAN ISO is damaged: its files cannot be read correctly. Copy or download it again and compare its SHA-256 checksum.";
    let raw = std::fs::read(root.join(need[0])).map_err(|e| format!("{DAMAGED} ({e})"))?;
    let id = String::from_utf8(raw).map_err(|_| DAMAGED.to_string())?.trim().to_string();
    if !id.starts_with("MAN-") || id.len() > 64 || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.') {
        return Err(DAMAGED.into());
    }
    let (mut bytes, mut files) = (0u64, 0u64);
    walk(root, &mut |_, meta| {
        bytes += meta.len();
        files += 1;
        Ok(())
    })?;
    Ok(IsoInfo { path: String::new(), id, bytes, files })
}

fn walk(dir: &Path, f: &mut dyn FnMut(&Path, &std::fs::Metadata) -> Result<(), String>) -> Result<(), String> {
    let mut entries: Vec<_> = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let p = e.path();
        let meta = e.metadata().map_err(|e| format!("{}: {e}", p.display()))?;
        if meta.is_dir() {
            walk(&p, f)?;
        } else {
            f(&p, &meta)?;
        }
    }
    Ok(())
}

/// Every file of `src` exists in `dst` with the same size and the same
/// live/man-id (Windows adds its own folders to a new drive; those are fine).
pub fn verify_copy(src: &Path, dst: &Path) -> Result<(), String> {
    walk(src, &mut |p, meta| {
        let rel = p.strip_prefix(src).map_err(|e| e.to_string())?;
        match std::fs::metadata(dst.join(rel)) {
            Ok(m) if m.len() == meta.len() => Ok(()),
            _ => Err(format!("The copy of MAN on the installer partition is incomplete ({} is missing or damaged).", rel.display())),
        }
    })?;
    let a = std::fs::read(src.join("live/man-id")).map_err(|e| e.to_string())?;
    let b = std::fs::read(dst.join("live/man-id")).map_err(|e| e.to_string())?;
    if a != b {
        return Err("The copy of MAN on the installer partition does not match the ISO.".into());
    }
    Ok(())
}

/// Copy every file of `src` into `dst`, reporting progress by bytes.
pub fn copy_tree(src: &Path, dst: &Path, total: u64, report: &dyn Fn(Progress)) -> Result<(), String> {
    use std::io::{Read, Write};
    let started = std::time::Instant::now();
    let mut done = 0u64;
    let mut last = std::time::Instant::now();
    let mut buf = vec![0u8; 4 << 20];
    walk(src, &mut |p, _| {
        let rel = p.strip_prefix(src).map_err(|e| e.to_string())?;
        let out = dst.join(rel);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        let mut r = std::fs::File::open(p).map_err(|e| format!("{}: {e}", p.display()))?;
        let mut w = std::fs::File::create(&out).map_err(|e| format!("{}: {e}", out.display()))?;
        loop {
            let n = r.read(&mut buf).map_err(|e| format!("{}: {e}", p.display()))?;
            if n == 0 {
                break;
            }
            w.write_all(&buf[..n]).map_err(|e| format!("{}: {e}", out.display()))?;
            done += n as u64;
            if last.elapsed().as_millis() >= 250 {
                last = std::time::Instant::now();
                let speed = done as f64 / started.elapsed().as_secs_f64().max(0.001);
                report(Progress::Detail(done as f64 / total.max(1) as f64, format!("{} of {}  ·  {}/s", human(done), human(total), human(speed as u64))));
            }
        }
        w.sync_all().map_err(|e| format!("{}: {e}", out.display()))?;
        Ok(())
    })?;
    report(Progress::Detail(1.0, format!("{} copied", human(done))));
    Ok(())
}
