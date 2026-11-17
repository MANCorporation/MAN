# MAN for Apple Silicon Macs

MAN on M1, M2 and M3 Macs, with the drivers and boot loader of
[Asahi Linux](https://asahilinux.org). M4 and newer are not supported by Asahi yet
(no installer, no GPU/display/USB drivers), so neither by MAN.

## How a Mac gets MAN

Apple Silicon Macs cannot start a PC-style ISO: only macOS or recoveryOS can register
another system. Like Fedora Asahi Remix, MAN is installed from macOS:

    curl -L https://github.com/MANCorporation/MAN/releases/download/<tag>/man-mac.sh | sh

`man-mac.sh` is Asahi's bootstrap script pointed at MAN's release. It downloads the
Asahi installer (MAN's copy, `installer-v0.9.2-man3.tar.gz`), which shrinks macOS if
asked, creates the partitions, copies the Mac firmware from macOS into the EFI
partition (`vendorfw/`), installs m1n1 stage 1 and asks the user to confirm it once in
recoveryOS. After that the Mac boots MAN through m1n1 → U-Boot → GRUB, and MAN Setup
runs on the first start, like on a PC.

## Boot chain

| Part | Where it comes from | Where it lives |
| --- | --- | --- |
| m1n1 stage 1 | Asahi installer | macOS stub partition (boot picker entry, MAN icon) |
| m1n1 stage 2 + device trees + U-Boot | `make-package.sh` (`esp/m1n1/boot.bin`), later `update-m1n1` | EFI partition |
| GRUB | MAN installer (`grub-install --removable`) | EFI partition `EFI/BOOT/BOOTAA64.EFI` |
| kernel, initramfs | Asahi kernel built here | `/boot` (ext4) |
| firmware | the Mac's own macOS, by the Asahi installer | EFI `vendorfw/`, loaded by the initramfs (asahi-scripts) |

## On the Mac (MAN side)

- Startup Disk (`root/usr/bin/man-startup-disk`, GTK 4/libadwaita): Start MAN / Show a
  start menu / Start macOS, and Restart to macOS. Changes go through
  `/usr/libexec/man-apple-boot` (pkexec, polkit `com.man.apple-boot`), which uses
  `asahi-bless` (Asahi, MIT; `build-bless.sh`) on the Mac's NVRAM.
- "Restart to macOS…" in the power menu: extension `man-mac@man.invalid` (only on Macs).
- "Show a start menu": `/etc/grub.d/44_man_macos` adds a GRUB entry "macOS"; it sets
  `man_next=macos` in grubenv and starts MAN, whose `man-apple-next.service` sets macOS for
  the next start and restarts.
- DUHACH! for Mac: `duhach/mac` (README there); also on the stick's MAN_MAC partition.

## Building

1. **Kernel** `build-kernel.sh` → `lfs/apple/kernel/`. Source: `lfs/apple/linux`
   (`git clone --depth 1 --branch asahi-7.3.3-2 https://github.com/AsahiLinux/linux`).
   Config: MAN's ARM64 config + `asahi.config` (16K pages, Rust GPU driver); disk and
   SPI keyboard built in; drivers that need Mac firmware (M2+ keyboard over
   DockChannel, xHCI/DWC3 USB) are modules, loaded after the firmware.
   Tools: LLVM 19 + libclang in `tools/llvm`, flex/bison/m4 in `tools/hostbin`, rustup.
2. **U-Boot** `build-uboot.sh` → `lfs/apple/ub/u-boot-nodtb.bin` (AsahiLinux/u-boot
   `asahi-v2026.07-3`, Debian's GCC cross compiler unpacked in `tools/gcc-arm64`).
   Debian's `u-boot-asahi` (2025.01) is too old for M3.
3. **System** In the ARM64 builder (`dev-tools/arm64/builder.sh` with
   `TARGET=lfs/apple/man-gnome-apple.img EXTRA=lfs/apple/disk.img`, the image being a copy
   of `lfs/arm64/man-gnome-arm64.img` and `disk.img` an empty 14 GiB file), with
   `lfs/arm64/share/apple/` holding `kernel/`, `u-boot-nodtb.bin`, `root/` (= `dev-tools/apple/root`)
   and the `guest/` scripts:
   - `guest/apple-system.sh`: Asahi kernel instead of the generic one; Debian's
     asahi-scripts, asahi-fwextract, m1n1, speakersafetyd, asahi-audio, tiny-dfr;
     U-Boot; MAN's Mac services; initramfs.
   - `guest/apple-install.sh`: MAN's own installer backend, "Erase Disk" onto `vdc`
     (EFI + ext4 /boot + Btrfs @system/@home/@snapshots, MAN Setup on first start),
     copied without Btrfs compression so LZMA packs it small.
4. **Package** `make-package.sh "10 TEST 6" https://github.com/MANCorporation/MAN/releases/download/v10-test6`
   → `release/apple/`: `MAN-<ver>-apple.zip` (esp/, boot.img, root.img, man.icns;
   images LZMA-compressed, must stay under GitHub's 2 GiB), `installer_data.json`,
   `installer-v0.9.2-man3.tar.gz`, `latest`, `man-mac.sh`. Upload all of them to the
   release; the URL given must be that release's download URL.
5. **VM test** `test-vm.sh` boots `disk.img` (overlay) in QEMU with `-cpu max` (16K pages
   work). It checks GRUB, kernel, initramfs, systemd, MAN Setup and the desktop, not
   the Mac drivers.

## MAN's Mac services (`root/`)

- `man-apple-grow.service`: once, `btrfs filesystem resize max /` (the Asahi installer
  enlarges the partition to the size the user chose).
- `man-apple-m1n1.service`: each start, rebuild `m1n1/boot.bin` from the running system
  and write it only when it changed, so a `.upd` with a new kernel also brings matching
  device trees to the EFI partition.

Both only run when `/proc/device-tree/chosen/asahi,efi-system-partition` exists (a Mac
started by m1n1).

## Changes to the Asahi installer

`installer/patch-urlcache.py`: HTTP status checks and re-resolving GitHub's signed
redirect links during long downloads. `installer/patch-auto.py`: shuts down / restarts
by itself. `installer/patch-man-menu.py`: with `MAN_AUTO=1` (set by the stick's
`install-man.sh`, also `DISTRO=MAN`) Asahi's partition list and questions are replaced
by MAN's menu:

1. only the MAN boot loader (stub macOS + ESP, OS entry `man_mode` 1),
2. boot loader + empty "MAN Boot" (1 GiB) and "MAN" partitions (`man_mode` 2), asks the size,
3. only the two empty partitions (boot loader already there), asks the size.

Space comes from free space, else macOS's APFS container is shrunk (never below Asahi's
limits: 38 GB stays free in macOS). With 1 then 3 the MAN partitions sit right after
macOS, before the boot loader's partitions; the live installer (`probe.rs` `mac_pair`)
finds them in either place. Everything else is Asahi's installer v0.9.2 as published
(MIT licence).

Testing the menu without a Mac: a simulated Mac (fake `diskutil`, macOS and system
info, real `main.py`/`osinstall.py`) printed every disk change for all three choices
and the error cases.

## Licences

Asahi kernel (GPL-2.0), U-Boot (GPL-2.0+), m1n1 (MIT, Debian package), Asahi installer
and scripts (MIT). The kernel and U-Boot sources (exact tags above) must be offered with
each release, like the other GPL sources of MAN. Mac firmware is never distributed by
MAN: the installer copies it from the user's own macOS.

## Pitfalls

- `cp -a dir/. /target/` copies dir's owner and mode onto `/`: systemd then refuses
  "unsafe path transition" and accounts-daemon fails (black screen). Use tar with
  `--no-same-owner --no-overwrite-dir`.
- `mount` prints "would change RO state" warnings on stdout: GRUB scripts must send
  both outputs to /dev/null (fixed in `43_man_windows`).
- In the test VM, the virtio GPU stays dark after the screen blanks (mutter: atomic
  commit fails). VM only; restart gdm or disable blanking.
- Built-in drivers cannot get Mac firmware (it is only loaded by the initramfs).
- USB-C on M1/M2 Macs needs `thunderbolt_apple` (the Type-C controllers wait for its
  Thunderbolt switch; without it they stay in "deferred" and no USB works), plus
  `dwc3-apple`, `phy-apple-atc`, `tps6598x`: all in the initramfs (`hooks/man-live`).
- asahi-scripts' `init-top/asahi` mounts the ESP with `mount PARTUUID=...`, which
  BusyBox's mount does not understand: no firmware. MAN's fixed copy is in
  `root/etc/initramfs-tools/scripts/init-top/asahi` (uses util-linux blkid).
- If the live stick is not found, `scripts/manlive` writes `man-live-log.txt` (dmesg,
  deferred devices, modules) to the Mac's MAN EFI partition; read it in macOS with
  `diskutil mount disk0sN` (the "EFI - MAN" partition).
