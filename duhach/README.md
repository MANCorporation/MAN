# DUHACH!

Installs MAN from Windows, without a USB stick. Rust + GTK 4 + libadwaita
(GNOME look), cross-compiled on Linux.

```bash
duhach/build-windows.sh     # -> release/DUHACH!-<version>-windows-x64.zip (folder: DUHACH!.exe + DLLs + share/)
cargo run --release         # on Linux: demo mode, nothing is changed (DUHACH_DEMO_PREPARED / _SECUREBOOT / _BITLOCKER / _FAIL=<step>)
```

## What it does on Windows

1. Checks: administrator, UEFI, GPT disk, Secure Boot off (button to restart
   into UEFI settings), free space (MAN ≥ 20 GB, Windows keeps ≥ 10 GB),
   BitLocker and battery warnings, MAN already installed, **storage mode**:
   an Intel RST / AMD RAID controller (PCI class 0104, not Intel VMD) hides
   the disk from Linux, so DUHACH! blocks and offers **Switch to AHCI…**:
   `bcdedit safeboot minimal` + a RunOnce `*DUHACH! AHCI` entry that clears
   it and restarts, then restart into the UEFI settings (`shutdown /r /fw /f`,
   plain restart + F2/Del hint where unsupported). The user sets RAID → AHCI;
   Windows starts once in Safe Mode (loads the AHCI driver) and then
   normally. Tested in the VM: Safe Mode once, then normal, settings cleared.
2. Verifies the MAN ISO (mounts it; needs `live/man-id`, the squashfs, kernel,
   initrd and `EFI/MAN-Installer/grubx64.efi`).
3. "Make Room": slider for MAN's space, bar of the disk.
4. Prepare (after "I have a backup" + confirmation):
   shrink C: → FAT32 partition `MAN_LIVE` at the end of the freed space →
   copy and verify the ISO files → `bcdedit /copy {bootmgr}` entry
   "MAN Installer" with `\EFI\MAN-Installer\grubx64.efi`, added last and set
   as the one-time `bootsequence` → BitLocker suspended for 3 restarts →
   startup task "DUHACH! cleanup" (removes the boot entry once MAN deleted the
   partition; Windows would otherwise restore it from its BCD at every start).
   On any error everything is undone (partition removed, C: grown back).
5. Restart → MAN live session from the partition → installer offers
   "Install Alongside Windows" (man-installer 3.5).

Run again before installing: "Restart into the MAN Installer" or
"Remove and Give Space Back". State: `%ProgramData%\DUHACH!\state.json`.

## Layout

- `src/main.rs` pages, `src/sys.rs` PowerShell (`-EncodedCommand`) + demo backend
- `data/` icon (svg/ico), manifest (requireAdministrator), resource, `windows/` README, pixbuf loaders cache, cleanup.ps1
- `build-windows.sh`, `package-windows.sh`; toolchain in `../tools/mingw` (Debian mingw-w64, unpacked) and `../tools/gtk-win` (MSYS2 ucrt64 GTK 4.24 / libadwaita 1.10, `fetch.py`)
- `testvm/`: Windows 11 test VM (`unattend/` answer file + serial agent, `winvm.sh`, `winctl.py`, `flow.sh`, `swap.sh`, `type.py`, `serlog.py`)

## Tested (Windows 11 LTSC evaluation in QEMU/OVMF, 2026-10-20/21)

Checks, ISO verify, prepare, rollback after an error, prepare → remove (C:
back to full size, entry and task gone), restart → MAN live from the
partition, install alongside, MAN boot menu with Windows (chainload works),
first MAN start removes the partition and grows MAN, first Windows start
removes the leftover boot entry. Note for testing: QEMU `bootindex` makes OVMF
prune and reorder NVRAM entries (BootNext ignored); test without it
(`BOOTIDX= VGA=virtio testvm/winvm.sh ...`).

Real PC 2026-10-21 (ASUS, Intel RST RAID mode): the installer did not see the disk → this check. Not tested: Intel RST detection on real hardware, BitLocker-encrypted C:, Secure Boot (MAN's kernel is not
signed: it must be off).
