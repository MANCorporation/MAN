# MAN Installer 3.6

The MAN system installer (`Install MAN`), first-start setup (`MAN Setup`) and
snapshot manager (`Snapshots`).
Rust backend, GTK 4 + libadwaita user interface. Replaces the old Python
`man-installer`/`man-oobe` (1.x/2.x), which is removed from the live image by
the package upgrade.

## Disk layout

```
GPT
├─ MAN_EFI    1 GiB   FAT32, /boot/efi
├─ MAN_BOOT   1 GiB   ext4,  /boot          (always for "Erase"; lets MAN Setup encrypt later)
└─ MAN_ROOT   rest    Btrfs "MAN"  (inside LUKS2 "MAN_CRYPT" when encrypted)
     ├─ @system      /             the operating system
     ├─ @home        /home         personal files, never touched by rollback
     └─ @snapshots   /.snapshots   <id>/snapshot (read-only) + <id>/info.json
```

All subvolumes share the free space of the partition: no fixed sizes, no
quotas. Snapshots are Btrfs copy-on-write and use space only for what changed.
The system stays a normal, writable Linux system; root access is through
`sudo` (or an optional root password chosen in the installer).

## What it does

| Flow | How |
|---|---|
| Erase and install | Repartitions the chosen disk, copies the live system with rsync, configures it, installs GRUB (firmware entry plus the `EFI/BOOT/BOOTX64.EFI` fallback), takes a "Fresh installation" snapshot |
| Custom partitions | User picks the EFI, system and (with encryption) boot partitions; only those are formatted. *Disks* can be opened from the page |
| Reinstall | Saves the current `@system` as a "System before reinstalling" snapshot, replaces it, keeps `@home`; the same username keeps its home folder |
| Encryption | LUKS2 + cryptsetup-initramfs; the passphrase is asked at boot |
| Snapshots app | Create, delete and restore snapshots; daily and pre-update automatic snapshots with retention |
| Rollback | From the app (applied at next boot) or from the GRUB submenu *Restore a MAN snapshot*, which restores in the initramfs before `/` is mounted |

## MAN Setup (first start)

The installer no longer asks for an account. On the first start GDM logs in the
locked `man-oobe` account into the `man-oobe` session: a minimal mutter
compositor running only `man-oobe` fullscreen. Pages:

Language (74 languages: big globe over "Welcome" drifting in every language,
searchable list; the system's language is preselected) → Country (flags from
iso-codes, preselected from the language; sets the formats locale and, when no
time zone is set yet, the country's main time zone from zone.tab) → Licenses (must accept; full texts from /usr/share/common-licenses) →
Internet (NetworkManager status, Wi-Fi list and connect; skippable) → Account
(name, username, password, autologin, computer name, optional root password;
existing /home folders can be reused) → Customize (light/dark, 9 GNOME accent
colors, wallpapers from gnome-background-properties; applied live: style and
accent with a full-screen crossfade, wallpaper crossfades behind the card) →
Privacy (location services, automatic time zone) → Encryption → Preparing MAN
(spinner) → Reboot.

The backend (`OobeApply`) creates the account, writes the user's dconf database
directly (`dconf compile`, no D-Bus), points GDM at the new user, and removes
the setup account at the next boot (`man-oobe-cleanup.service`). Wallpaper
thumbnails are pre-generated at package installation
(`man-oobe --generate-thumbnails`).

**Encryption after setup** (needs the `MAN_BOOT` partition):
1. Setup shrinks Btrfs by 32 MiB, stores a one-time key on the disk and adds
   `man-encrypt` to the initramfs.
2. At the restart, the initramfs starts LUKS2 encryption in place
   (`cryptsetup reencrypt --encrypt --init-only --reduce-device-size 32M`, a few
   seconds) and unlocks the disk with that key.
3. `man-encrypt.service` encrypts the rest online, in the background, at idle
   I/O priority, then shreds the key and rebuilds the initramfs. A restart in
   between is safe: the normal passphrase prompt appears and the job resumes.

Safety rules enforced by the backend, independently of the UI:

- Nothing is written before the user confirms in a dialog naming the disk.
- The plan carries a token (`path|size|serial`) of the confirmed disk; if the
  disk changed, the backend refuses and modifies nothing.
- The disk MAN runs from, read-only disks and mounted partitions are refused.
- Every rollback first saves the current system as a snapshot; the replaced
  system is removed only after a later healthy boot (`man-boot-cleanup.timer`).
- `/boot` and `/boot/efi` are mounted with `nofail`, and the emergency shell
  opens even though root is locked, so a broken boot can always be repaired.

## Languages (3.6.0, MAN 10.5)

The list is `crates/man-common/src/languages.rs` (locale, native and English
name, "Welcome", main country). The image has `locales`, `libc-l10n` and Noto
fonts for every script, and all 74 locales pre-generated; any other locale
(e.g. formats `de_AT` for German in Austria) is generated on demand
(`man-backend/src/locale.rs`, `localedef`).

- Installer, first page: language button → dialog → `SetLiveLanguage` writes
  `/etc/default/locale` and the MANInstaller account's AccountsService language,
  then restarts GDM; autologin brings the live session back in that language.
  The install copies the live language (`Plan.language`).
- MAN Setup: `OobeConfig.language`/`country` → system `LANG`, the user's
  AccountsService language, dconf `org.gnome.system.locale region`, time zone.
- MAN's own apps (installer, MAN Setup, Snapshots, Updates) are still English
  only; GNOME and its apps are translated.

## App Installer (3.10.0, MAN 10.10)

`man-app-installer` (com.man.AppInstaller, the default app for these files)
installs an app from a downloaded file, with the steps of a classic
installer on the left (Introduction, License, Destination Select,
Installation Type, Installation, Summary) and the app's icon below them:

- **.deb**: name, icon, description and license (`/usr/share/doc/PKG/copyright`)
  are read from the package (`dpkg-deb`); `apt-get -s` shows what else it
  needs; installing runs `man-app-installer-helper` through pkexec (polkit
  `com.man.apps.install`): `apt-get update`, then `apt-get install ./file.deb`.
  **Keep After MAN Updates** (on by default): the helper saves the package
  as `/var/lib/man/kept-debs/<name>.deb` and the packages apt downloaded for
  it under `archives/`. Every update reinstalls the kept packages that are
  still installed inside the new system before the restart (apt in the
  chroot, the saved files as its download cache, so usually offline); if
  that fails, `man-kept-debs.service` retries at the next start
  (`man-app-installer-helper reinstall-kept`). Removing the app with apt
  makes the next update forget it.
  Missing dependencies come from Debian sid (`/etc/apt/sources.list.d/man-debian.sources`)
  with **Pin-Priority 50** (`/etc/apt/preferences.d/man-debian`): apt only adds
  packages that are not installed and never upgrades MAN's own. Apps from
  .deb files are part of the system, so a MAN update removes them (the
  installer says so).
- Flatpak "For All Users" runs `flatpak install --system` as root through
  the helper (Flatpak's own helper cannot ask for a password in
  non-interactive mode: "ConfigureRemote not allowed"). The export folders
  (`/var/lib/flatpak/exports/share/…` via tmpfiles.d, `~/.local/share/flatpak/exports/share/…`
  via user tmpfiles, `systemd-tmpfiles-setup.service` enabled globally) exist
  before GNOME starts, so new apps show up without logging out.
- **.flatpak** bundles (metadata, AppStream and icon read straight from the
  bundle's GVariant header) and **.flatpakref** files: `flatpak install
  --user` ("Only for Me") or `--system` ("For All Users").
- **AppImages**: the icon and menu entry are taken from the AppImage
  (`unsquashfs` at the offset after the ELF runtime; nothing is run). Only
  for me: moved to `~/AppImages`, entry `~/.local/share/applications/appimage-ID.desktop`;
  for all users (through the helper): `/opt/AppImages` and
  `/usr/local/share/applications` (both kept by MAN updates). libfuse2t64
  is installed so older AppImages run.

## Layout of this directory

```
crates/man-common    shared types (requests, events, plan, snapshot config)
crates/man-backend   man-installer-backend: probe, install, snapshots (runs as root via pkexec)
crates/man-gui       man-installer and man-snapshots (GTK 4 / libadwaita 1.6+)
data/                polkit, desktop, metainfo, icons, systemd, GRUB, initramfs, apt hook
packaging/build-debs.sh   builds man-system-tools_*.deb and man-installer_*.deb
```

Backend protocol: `pkexec man-installer-backend request` reads one JSON
`Request` on stdin and writes JSON-line `Event`s. Full log:
`/var/log/man-installer.log`.

## Building

Needs Rust and the GTK 4 / libadwaita development files:

```bash
sudo apt install libgtk-4-dev libadwaita-1-dev     # once, on the build host
cargo test --workspace
packaging/build-debs.sh                            # -> target/debs/*.deb
```

## Trying it

```bash
./run-gnome.sh install-test    # live session + empty 40 GB virtual disk (lfs/install-target.qcow2)
./run-gnome.sh installed       # boot the test disk without the live "USB stick"
```

The live session boots like from a USB stick (`lfs/man-live-boot.img`, built by
`make-live-boot.sh` with GRUB 2.14 from `tools/`; rebuilt automatically when the
kernel changes). The stick has boot priority, like a forgotten USB stick. Its
menu looks for an installed MAN on the other disks (`MAN_BOOT` label for
encrypted installs, `/@system/boot/grub/grub.cfg` otherwise) and, when found,
shows that system's own boot menu with its 3-second countdown, plus *Try or
Install MAN (live session)* at the bottom. So "Restart Now" after installing
starts the new system, inside the same QEMU window.

Delete `lfs/install-target.qcow2` to start over with an empty disk.

The downloadable ISO is described in `../ISO.md` (`./run-gnome.sh iso`). Started
from the ISO, the root is a RAM overlay; the installer reads the system size
from `/run/live/medium/live/filesystem.size` and treats the ISO medium
(label `MAN_LIVE`) as the live disk.

3.4.0 also ships `20_man-desktop.gschema.override` (PCD in the dock, GNOME's
donation pop-up off) and the MAN logo (`man-logo*`, `LOGO=man-logo` in
os-release, shown in Settings → About).

## Tested in QEMU (UEFI, OVMF)

- Erase install (plain Btrfs) → boots, login, Snapshots app, settings change,
  manual snapshot, restore from the app, restart.
- Erase install with encryption → passphrase at boot, login, home file created.
- Reinstall of the encrypted system → home file and password change kept,
  old system available as a snapshot.
- Restore of "System before reinstalling" from the GRUB menu → boots, the
  replaced system is cleaned up afterwards.
- Custom partitions on a pre-partitioned second disk → boots, login.
- Refusals: changed disk (stale confirmation), the running live disk.
- Fallback boot loader after the firmware boot entry no longer matched.

## Install Alongside Windows (3.5.0, for DUHACH!)

When a disk has Windows (NTFS or Microsoft reserved partition) and at least
system size + 4 GB + 1 GB of unpartitioned space, the installer offers
**Install Alongside Windows** (preselected). It creates MAN_BOOT and the
system partition in the largest free region, shares the existing EFI
partition without formatting it and never replaces Windows' fallback boot
loader. `/etc/grub.d/43_man_windows` adds a "Windows" entry; the menu waits
5 s. Started from a DUHACH! partition (FAT `MAN_LIVE` on an internal disk),
only that partition is protected, not the whole disk, and the install writes
`/etc/man/duhach-installer`: on MAN's first start `man-duhach-cleanup`
deletes the partition, the "MAN Installer" firmware entry and grows the
system partition into the space. See ../duhach/README.md.
