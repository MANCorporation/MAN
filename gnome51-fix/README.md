# GNOME 51 desktop fix (2026-10-18, second pass)

The earlier `GNOME51_REPAIR.md` got GDM's login screen working, but logging in
never produced a usable desktop. Root causes found in `lfs/man-gnome.img`:

| Problem | Effect | Fix |
|---|---|---|
| `/etc/profile` was the LFS *build-chroot* profile (`export HOME=/root`, `LC_ALL=POSIX`, `/tools/bin` PATH, no `profile.d`) | Every GNOME session ran with `HOME=/root`; apps could not write settings/cache; Console, portals, search providers failed | Replaced with Debian's `/usr/share/base-files/profile` (`profile` here) |
| `man` and `root` had locked passwords (`*`) | Nobody could log in | Password set to `man` for both (hash generated in the LFS chroot) |
| `/usr/bin/login`, `/usr/sbin/nologin` missing | Text-console login looped forever | Installed Debian trixie `login` package (`login_*.deb`, util-linux 2.41.5, matches the image) |
| 99.8% of files owned by UID 1000 (= the desktop user `man`) from rootless extraction; postinst permissions never applied | `man` could overwrite the OS; `sudo`, `pkexec`, polkit/dbus helpers, `fusermount3` broken | `man-repair.sh` run as real root **inside the VM**: chown to root, then owners/modes from .deb metadata, `statoverride` and postinst scripts (`special-perms.tsv`), setcap, `man` added to `sudo` |
| Kernel rebuilt with host gcc: lost `CONFIG_RUST`; never had `CONFIG_FUSE_FS` | No `/dev/fuse` → document portal / Flatpak / GVFS broken | `05-kernel-gnome51.sh` rebuilt 7.3-rc7 in the LFS chroot (`~/mandroid_mount`, gcc 15.2 + rustc 1.99) from the Rust config + USER_NS, FUSE, BPF, MEMCG/PSI, overlayfs, squashfs, fanotify, uinput, landlock |
| `/etc/hosts` still named `mandroid` | `sudo: unable to resolve host man` | `127.0.1.1 man` |
| `gnome-initial-setup` binary not installed but its first-login unit enabled | Failed unit on each login | `~/.config/gnome-initial-setup-done` for `man` |

Verified: GDM → login as `man` → GNOME Shell 51 desktop, Console and Files open,
`HOME=/home/man`, `sudo` works, xdg-desktop-portal + document portal active,
bubblewrap sandboxes work, `systemctl --failed` empty, ext4 fsck clean.

Backups: `lfs/man-gnome.before-desktop-fix-20261018.img`,
`lfs/boot/*-7.3-rc7.norust-20261018`.

Known leftovers (cosmetic): `gnome-software.service` reports failed because
GNOME Software is already running as the session's background service; the
ONNX GStreamer plugin has an unresolved symbol; the greeter shows the Debian logo.

## Live user + autologin
`live-user.sh` (run as root in the guest) creates `MANInstaller` (UID 1001, no
password, passwordless sudo via `/etc/sudoers.d/90-maninstaller`) and sets
`AutomaticLoginEnable=true` / `AutomaticLogin=MANInstaller` in
`/etc/gdm3/daemon.conf`. Backup before this change:
`lfs/man-gnome.before-autologin-20261018.img`.

## GNOME 51 app set (GNOME 48 cleanup)
`apps51.sh` (root, in the guest, with a local apt repo of the GNOME 51 archive
mounted over virtio-9p) makes the preinstalled apps match the GNOME 51 core app
set from `gnome-build-meta-51` (`meta-gnome-core-apps.bst`):

- Registers the 219 GNOME 51 overlay packages in dpkg at their exact overlay
  versions (`apps51-overlay-pins.txt`): the dpkg database now matches the files.
- Installs the GNOME 51 app/shell versions (`apps51-install.txt`), including
  apps whose 51 files were on disk but registered as 48.
- Removes GNOME 48 leftovers (`apps51-remove.txt`): man-desktop (GNOME 48 meta),
  gnome-core 48, Mutter 48 libs, libgnome-desktop-3, gnome-menus 3.36, Orca 48,
  gnome-software-plugin-deb 48, libyelp0 42, stale ONNX/OpenEXR/gusb libs and the
  Python 3.13-only modules that could not load under the overlay's Python 3.14.
- Deletes 4,797 unregistered GNOME 48 files of Evince, Totem and GNOME Terminal
  (replaced in GNOME 51 by Papers, Showtime and Console).
- Installs `hostname` (missing; ssl-cert/cups postinst need it).

Fixed along the way: GtkSourceView 5.22 + libspelling 0.4.10 + enchant 2.8.16
(Text Editor segfaulted in enchant 2.8.2 plugins under the 2.8.16 library).

Deviations from upstream GNOME 51, because no matching build exists locally or
in Debian: Papers 49.8 (the MAN 51.0 build lacked its 51 libraries and crashed),
Sushi 50.0, Orca absent. Image grown to 12 GB (sparse). Backup:
`lfs/man-gnome.before-gnome51-apps-20261018.img`.
