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

## Updates from GitHub (MAN 10.7, 2026-10-24)

MAN finds updates in the **GitHub releases** of
https://github.com/MANCorporation/MAN:

- `man-update-check` (man-system-tools 3.8.0, uses `curl`) reads
  `https://api.github.com/repos/MANCorporation/MAN/releases`, ignores drafts
  and pre-releases, and offers the newest asset named `MAN-<version>.upd`
  that is newer than `/usr/lib/man/version`. A `MAN-<version>.upd.sha256`
  asset, if present, is checked after the download. The file is saved in
  `~/Downloads` (a partial download resumes) and opened in MAN Update, which
  verifies the ed25519 signature: an unsigned or altered file is refused.
- **Settings → Updates** checks every time it is opened.
- `man-update-check.timer` (user timer, enabled globally) runs
  `man-update-check --notify` 10 minutes after login and then daily, and
  shows one notification per new version ("Open Updates" opens Settings →
  Updates). Off in the live session; switch in Settings
  (`com.man.Updates auto-check`).
- Testing: `MAN_UPDATE_API=http://host:port` points the checker at a fake
  releases API (a JSON list like GitHub's).

### Publishing a release

```bash
gh release create v10.7 --repo MANCorporation/MAN --title "MAN 10.7" \
    --notes-file lfs/updates/MAN-10.7.NOTES.txt \
    release/MAN-10.7.upd release/MAN-10.7.upd.sha256 \
    release/MAN-10.7-amd64.iso release/MAN-10.7-amd64.iso.sha256
```

GitHub allows at most **2 GiB per file**: the .upd (1.96 GB) and the ISO
(1.9 GB) fit, but a bigger system needs a smaller payload (or splitting,
which man-update-check does not support yet). The release notes on GitHub
are what Settings shows under "What's New".

## MAN 10.9 (2026-10-24)

Updates install inside Settings → Updates (Settings 1:51.0-1man5, see
src/README.md); main buttons stay in a fixed bar at the bottom. Tested in
QEMU on installed systems: 10.7 → 10.8 from the (fake) GitHub release with a
snapshot, including a cancelled password prompt + Try Again and leaving and
reopening the panel while preparing; 10.8 → 10.9 in Czech from a file in
Downloads without a snapshot. Both ended with "Your System Was Updated".

## TEST builds and Beta Updates (MAN 10.10, 2026-10-24)

Versions are either **releases** ("10.10") or **TEST builds** ("MAN 10 TEST 5":
major version and test number). A TEST build also records the release it was
made after (its *base*), so the order is

    10.9 < 10 TEST 4 (made after 10.9) < 10 TEST 5 < 10.10 < 10 TEST 6 (made after 10.10)

(`man_common::man_version_cmp`; the backend, `man-update-check` and MAN
Update all use it).

- On a system: `/usr/lib/man/version` = `10 TEST 5`, `/usr/lib/man/version-base`
  = `10.9` (only on TEST builds). Set both, os-release and the Beta Updates
  default with **`man-set-version`** (man-system-tools) inside the image:
  `man-set-version 10.10` or `man-set-version "10 TEST 5" 10.9`. TEST images
  get `/usr/share/glib-2.0/schemas/90_man-test-build.gschema.override`
  (`beta-updates=true`), releases have it removed.
- The signed manifest has `"base"` for TEST builds:
  `man-mkupd pack --version "10 TEST 5" --base 10.9 …` (refused without it).
  An update also writes version-base into the new system (or removes it).
- File names use dashes: `MAN-10-TEST-5.upd`, `MAN-10-TEST-5-amd64.iso`.
- **Settings → Updates → Beta Updates** (`com.man.Updates beta-updates`):
  off = only GitHub releases; on = pre-releases too; the newest of both that
  is newer than the system is offered (a TEST build shows "This is a TEST
  version"). Changing the switch checks again.
- Systems older than 10.10 do not know TEST versions: release 10.10 first.

### Publishing a TEST build (pre-release)

The release notes must name the base in an HTML comment (hidden on GitHub,
removed from "What's New"):

```bash
{ cat lfs/updates/MAN-10-TEST-1.NOTES.txt; echo; echo '<!-- man-base: 10.10 -->'; } > /tmp/notes.md
gh release create v10-test1 --repo MANCorporation/MAN --prerelease --title "MAN 10 TEST 1" \
    --target <full sha> --notes-file /tmp/notes.md \
    release/MAN-10-TEST-1.upd release/MAN-10-TEST-1.upd.sha256 \
    release/MAN-10-TEST-1-amd64.iso release/MAN-10-TEST-1-amd64.iso.sha256
```

## MAN 10 is in testing (2026-10-25)

MAN 10 ships as TEST builds only: GitHub pre-releases "MAN 10 TEST n", made
after 10.10 (`man-set-version "10 TEST n" 10.10`, `man-mkupd pack --base 10.10`).
The releases 10.6–10.10 were set back to **drafts** (hidden, kept; `gh release
edit vX --draft=false` publishes one again). Consequences:

- Machines see TEST builds only with Settings → Updates → Beta Updates on
  (default on TEST builds; a 10.10 install must switch it on once).
- 10.9 and older do not understand TEST versions.
- Release names (from MAN 10 on): **MAN 10**, small updates **10a, 10b … 10z,
  10aa**, medium **11**, big **20**. Order (man_version_cmp, man-installer
  3.10.2): 10.9 < 10.10 < 10 TEST n < 10 < 10a < 10b < 11 TEST n < 11 < 20.
  Systems before that code (10.10, TEST 1, TEST 2) use the old base rule and
  would see "10" as older than their TEST build, so the **next TEST build must
  contain 3.10.2** before "MAN 10" is published.

## MAN 10 TEST 2 (2026-10-25)

App Installer fixes: Flatpak for all users installs as root through
man-app-installer-helper; Flatpak export folders exist at login; `.deb` apps
with "Keep After MAN Updates" are reinstalled inside every update
(`reinstall_kept` in update.rs, from `/var/lib/man/kept-debs`). Tested:
xterm (+ 2 dependencies from Debian) kept through 10.10 → 10 TEST 1 with
"Need to get 0 B", i.e. offline.

## MAN 10 TEST 3 (2026-10-25)

MAN Sample Music (MANSampleMusic/, 3 MP3s + Artwork.jpg as cover.jpg) in
`/etc/skel/Music/MAN Sample Music` (new users) and the live user's Music.
man-installer 3.10.2 (release names 10, 10a, 11, 20). Payload 1.99 GB — only
~157 MB left under GitHub's 2 GiB asset limit.
