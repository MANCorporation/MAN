# GNOME 51 repair record

Verified on 2026-10-18 against `lfs/man-gnome.img`.

## Result

The image boots kernel 7.3-rc7, reaches `graphical.target`, starts GDM 51,
creates its dynamic greeter user, and runs GNOME Shell 51 as a Wayland display
server. The final diagnostic boot reported zero failed systemd units.

Start it with:

```bash
./run-gnome.sh graphical
```

The pre-repair disk is retained as
`lfs/man-gnome.pre-gnome51-repair-20261018.img`.

## What was broken

The current failure was not a kernel panic. An older log contained a root-mount
panic, but the present kernel already includes the required virtio and ext4
support. The black screen came from an inconsistent userspace overlay:

- GDM 51 was wrapped by local scripts that forced its dynamic `gdm-greeter`
  account to a different static UID.
- NSS did not include the systemd provider, so dynamic users could not be
  resolved consistently.
- Several GDM and user-manager systemd drop-ins attempted to compensate for
  that UID mismatch and made session startup less coherent.
- 144 symlinks embedded the host staging prefix and therefore pointed nowhere
  inside the guest.
- Rootless Debian package extraction left privileged programs such as `mount`
  owned by the build user. Because the setuid bit was retained, invoking
  `mount` as root changed its effective UID to 1000, causing every systemd mount
  unit to fail with “must be superuser.”
- The configured locale was absent from the image.

## Repair invariants

- GDM files come directly from `gdm3_51.0-1_amd64.deb`; no preload shim or
  binary wrapper is active.
- NSS account lookups use both the `files` and `systemd` providers.
- GDM owns the lifecycle of its dynamic greeter account.
- Guest symlinks contain guest paths, never the host staging path.
- Setuid/setgid binaries in the disk image have their required root/group
  ownership.
- The installed locale is `C.UTF-8`.

## Verification performed

- Booted a disposable qcow2 overlay backed by the repaired raw image.
- Confirmed `gdm.service`, the greeter's `user@.service`, the Wayland session,
  and `gnome-shell --wayland --mode=gdm --display-server` were running.
- Confirmed the virtual filesystems and `systemd-remount-fs.service` succeeded.
- Confirmed `systemctl --failed` returned no failed loaded units.
- Ran a final read-only ext4 check after modifying the image.

## Packaging caveat

GNOME 51 was installed by extracting Debian packages into an LFS root rather
than by a native MAN package transaction. The files are GNOME 51, but the dpkg
database still describes parts of the older base. Future upgrades must use the
overlay manifest at `lfs/usr/lib/man-build/gnome51-overlay-packages.tsv` and
preserve ownership, maintainer-script effects, sysusers entries, and symlinks.

## Second pass (later on 2026-10-18)

The first pass reached the GDM greeter only; logging in did not give a usable
desktop. See `gnome51-fix/README.md` for the root causes (build-chroot
`/etc/profile` forcing `HOME=/root`, locked passwords, missing `login`,
whole filesystem owned by UID 1000, kernel without FUSE and without Rust) and
the fixes. Login: user `man`, password `man`.

## GNOME 51 apps (2026-10-18)
The overlay packages are now registered in dpkg and GNOME 48 leftovers are
removed; the packaging caveat above no longer applies. See
`gnome51-fix/README.md` ("GNOME 51 app set").

## New installer (2026-10-18)
The Python installer and OOBE were replaced by MAN Installer 3.0 (Rust +
GTK 4/libadwaita) in `man-installer/`; see `man-installer/README.md`. The
kernel was rebuilt in the LFS chroot with Btrfs, dm-crypt, NVMe and a
framebuffer console (CONFIG_RUST still on), and `run-gnome.sh` now boots
through UEFI firmware. Backup before this change:
`lfs/man-gnome.before-installer-20261018.img`, `lfs/boot/*.before-installer-20261018`.
