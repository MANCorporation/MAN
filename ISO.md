# MAN ISO (for downloading)

```bash
./make-iso.sh                 # -> release/MAN-<version>-amd64.iso + .sha256
./run-gnome.sh iso            # try the newest ISO in QEMU (as a USB stick, plus the test disk)
```

Build it only when a release is wanted; everyday testing keeps using
`lfs/man-gnome.img` (`./run-gnome.sh graphical | install-test | installed`).
`make-iso.sh` reads the image read-only and needs no root (it uses
`tools/fuse2fs`, `tools/busybox`, the host's `mksquashfs`, `xorriso` and
`grub-mkrescue`, and `tools/mtools`). Shut the VM down first. It takes about
4 minutes.

## What is on it

| File | What |
|---|---|
| `live/filesystem.squashfs` | the image's system (zstd); caches, logs, `/tmp` and the machine id left out |
| `live/vmlinuz` | the MAN kernel from the image's `/boot` |
| `live/initrd.img` | busybox + `iso/init`: finds the medium, mounts the system with a RAM overlay, starts MAN |
| `live/filesystem.size` | uncompressed size, for the installer's free-space check |
| `live/man-id` | build id; the initramfs only accepts a medium with the id GRUB passed |
| `boot/grub/grub.cfg` | from `iso/grub.cfg` |
| `EFI/MAN-Installer/grubx64.efi` | GRUB for DUHACH!: finds `live/man-id` on a hard-disk FAT partition and shows the same menu |

Joliet names are included so Windows (DUHACH!) reads the files with their real names.

Volume label `MAN_LIVE`, so the installer never offers the stick it runs from.

Boot menu: *Try or Install MAN*, *(copy to memory)* (the stick can be removed
afterwards), *(safe graphics)* (`nomodeset`), *UEFI Firmware Settings*. If a
MAN installation is found on the computer, its own boot menu is shown instead,
so a forgotten USB stick still starts the installed system.

## Writing it to a USB stick

GNOME Disks → *Restore Disk Image…*, Fedora Media Writer, balenaEtcher, Rufus
(DD mode) or `sudo dd if=MAN-10.0-amd64.iso of=/dev/sdX bs=4M status=progress oflag=sync`.
It also works burned to a DVD.

## Tested (QEMU)

- UEFI USB stick: live session with autologin and the installer; file
  capabilities and a unique machine id per boot.
- Installer from the ISO: refuses the ISO stick itself; erase install to a
  blank disk (1 min 51 s); installed fstab, accounts and capabilities correct;
  restart with the ISO still attached starts the installed MAN (MAN Setup).
- UEFI DVD: live session.
- Legacy BIOS USB stick: live session (installing needs UEFI; the installer says so).

Not done: Secure Boot (the kernel is not signed; switch Secure Boot off),
real hardware (the kernel has only the drivers it was configured with).

## Kernel note

Needs `CONFIG_SQUASHFS_XATTR=y` (added 2026-10-20 with
`gnome51-fix/06-kernel-squashfs-xattr.sh`); without it the live system loses
file capabilities. Old kernel kept as `lfs/boot/*.before-squashxattr-20261020`.

## Storage drivers (2026-10-21)

After a real PC could not find the DUHACH! partition, the kernel got Intel
VMD (RST "RAID On" mode), eMMC/SD (SDHCI), UFS, USB UAS and more SAS/RAID
drivers, all built in (`gnome51-fix/07-kernel-storage.sh`; previous kernel
kept as `lfs/boot/*.before-storage-20261021`). Tested in QEMU: DUHACH!
partition on NVMe and on an SD card. If the medium is still not found, the
start-up screen now lists the disks it saw and, when there are none, says to
switch the storage mode from RAID/RST to AHCI.

## Laptop and desktop hardware (2026-10-22)

After the touchpad did not work on a real ASUS laptop, the kernel got 418
driver modules (`gnome51-fix/08-kernel-laptops.sh`; previous kernel kept as
`lfs/boot/*.before-laptops-20261022`): I2C touchpads/touchscreens (Intel LPSS,
DesignWare I2C, Intel/AMD pin controllers, I2C HID, ELAN, Synaptics RMI4),
Wi-Fi (Intel, Realtek rtw88/rtw89/rtl8xxxu, Atheros/Qualcomm ath9k-12k,
MediaTek mt76, Broadcom), Bluetooth, HD-Audio codecs and Intel/AMD SOF,
AMD/Intel Xe/NVIDIA (nouveau)/Radeon graphics, webcams, Realtek card
readers, ASUS/Lenovo/HP/Dell/Acer/MSI/Toshiba hotkeys, sensors, USB4. The
image has the Debian firmware packages (Intel/Realtek/Atheros/MediaTek/
Broadcom Wi-Fi, AMD/Intel/NVIDIA graphics, SOF audio, CPU microcode;
560 MB; not the Qualcomm phone/ARM firmware). The ISO is now 1.6 GB.
man-system-tools 3.5.1 moves live-boot's `kms`/`live` initramfs hooks aside:
they put nouveau/radeon and ~500 firmware files into every installed
system's initramfs (80 MB → 33 MB).
Tested in QEMU: modules load on their own (HD-Audio, e1000e), install from
the ISO, installed system boots. Touchpad/Wi-Fi need a real PC to test.

## File systems (2026-10-22)

The ASUS could not open an exFAT USB stick: exFAT and NTFS (ntfs3) are now
built into the kernel, HFS+/HFS, UDF, XFS, F2FS, CIFS and all code pages are
modules (`gnome51-fix/09-kernel-filesystems.sh`; previous kernel
`lfs/boot/*.before-fs-20261022`), and the image has exfatprogs and ntfs-3g
(formatting in Disks). Tested: exFAT and NTFS images mount, read and write.
