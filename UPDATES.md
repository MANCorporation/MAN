# MAN system updates (.upd)

## Trying it

```bash
./run-gnome.sh install-test     # install MAN (10.0) on the test disk, restart, finish MAN Setup
./run-gnome.sh installed        # boot it with the "MAN_UPDATES" USB stick attached
```

In Files, open **MAN_UPDATES** in the sidebar and double-click `MAN-10.1.upd`.
MAN Update opens: details and release notes → license → install location →
summary → "Create a snapshot first?" → progress → restart. The computer
restarts twice; after logging in a notice says "Your system was updated to 10.1".

## How it works

1. **Check:** `.upd` = tar with `manifest.json` (version, sizes, sha256 of the
   system), `manifest.sig` (ed25519), `LICENSE.txt`, `RELEASE-NOTES.txt`,
   `system.tar.zst`. Only files signed with a key in
   `/usr/share/man/update-keys/` are accepted; the payload checksum is
   verified while unpacking. Older or equal versions are refused.
2. **Snapshot** (optional): "Before update to MAN x.y", restorable from the
   Snapshots app or the boot menu.
3. **Update area:** the new system is unpacked into a Btrfs subvolume
   `@update` next to `@system`/`@home` (shares free space, no fixed size).
   Accounts, passwords, hostname, Wi-Fi, time zone, keyboard, GDM settings,
   fstab/crypttab, snapshot settings, Flatpak, `/usr/local` and `/opt` are
   carried over. Its initramfs and boot menu are prepared.
4. **Restart 1:** a one-time GRUB entry "Install MAN update x.y" boots; the
   initramfs swaps `@update` in as `@system` (old one kept as
   `@system.old-*`) and restarts. Nothing is copied at that point, so it
   takes seconds.
5. **Restart 2:** the new system starts, `man-update-finish.service` cleans
   up, and the login notice appears once. The old system is removed after a
   healthy start (`man-boot-cleanup.timer`).

`/home` is never touched. If anything fails before the restart, `@update`
is deleted and nothing changes. Software installed with apt is replaced by the
new system (use Flatpak or /usr/local/opt for software that should survive).

## Making an update

Signing key (made 2026-10-19): `keys/man-update.key` (**secret: back it up,
never share it**) and `keys/man.pub` (shipped in man-system-tools).

```bash
# 1. As root, on a copy of the system you want to ship (a mounted image or
#    tree), with VERSION_ID etc. changed as you like:
man-mkupd payload /path/to/system-tree outdir/

# 2. On your computer (license text: man-installer/packaging/UPDATE-LICENSE.txt,
#    replace @VERSION@):
man-mkupd pack --version 10.2 --payload outdir/ --key keys/man-update.key \
    --license LICENSE.txt --notes NOTES.txt -o lfs/updates/MAN-10.2.upd
man-mkupd verify lfs/updates/MAN-10.2.upd
```

`run-gnome.sh installed` rebuilds the updates stick automatically when a
newer `.upd` is in `lfs/updates/` (or run `./make-update-stick.sh`).
`man-mkupd` is built in `man-installer/target/release/` and installed on MAN
as `/usr/bin/man-mkupd`.

## Tested (QEMU, UEFI)

- Unencrypted 10.0 → 10.1 through the GUI (from Downloads and from the USB
  stick): version, os-release and new files present, home files, password,
  hostname and wallpaper kept, snapshot created, `@update`/old system cleaned,
  notice shown once.
- Encrypted 10.0 → 10.1: passphrase asked in the update boot and the normal
  boot, update applied, notice shown.
- Tampered `.upd` (changed version, original signature) refused.

## MAN 10.2 (2026-10-22)

`lfs/updates/MAN-10.2.upd` (also hard-linked as `release/MAN-10.2.upd`):
the laptop drivers and firmware (see ISO.md), PCD, MAN logo, smaller
initramfs. License and notes: `lfs/updates/MAN-10.2.LICENSE.txt` /
`MAN-10.2.NOTES.txt`. Made from the image with `/usr/lib/man/version` =
10.2 and VERSION_ID="10.2" in os-release (the image itself is now 10.2).
MAN-10.2.upd was remade the same day with exFAT/NTFS support. Tested (first build): a 10.0 install with its firmware and Wi-Fi modules deleted →
update_stage → restart → notice "updated to 10.2, previously 10.0",
firmware (564 MB) and drivers back, home files kept.

## MAN 10.3 (2026-10-22)

`lfs/updates/MAN-10.3.upd` (+ `release/`): Wi-Fi on the real ASUS did not
work. Causes: wpasupplicant (NetworkManager needs it for Wi-Fi) was not
installed, and Debian 13's firmware (20250410) is older than kernel 7.3's
Intel ucode API (100). Added wpasupplicant (+ libnl), firmware 20260916
(trixie-backports), iproute2 and iputils-ping; kernel module
mac80211_hwsim for testing (`gnome51-fix/10-kernel-hwsim.sh`). Tested in a
VM: two simulated radios, hostapd WPA2 access point, NetworkManager
connects (`nmcli dev wifi connect`), ping over Wi-Fi 0% loss. Image is 10.3.

## MAN 10.6 (2026-10-24)

`lfs/updates/MAN-10.6.upd` (+ `release/`, 1.96 GB, sha256 5ef2609a…):
man-installer / man-system-tools 3.7.0 — the MAN apps (installer, MAN
Setup, Snapshots, update installer) translated into all 74 languages
(71 catalogs, `man-installer/po/`, sources in `dev-tools/i18n/tx/`;
machine-written, native review welcome) and the MAN logo on the GDM login
screen (`/usr/share/gdm/dconf/95-man-logo`). Signature verified in a VM.
Note: the image's version lives in `/usr/lib/man/version` as well as
os-release — bump both (the update writes the version file on apply).
Old image kept as `lfs/man-gnome.before-10.6.img`.
