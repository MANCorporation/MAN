# MAN 10: what is left

## TEST 4 (published)
Cloud Drives added 2026-11-07 (man-installer 3.15.0 with rclone, Settings man9, desktop 1.4);
ISOs and `.upd` rebuilt with it:
- Settings → Cloud Drives below Online Accounts: all rclone providers, popular ones first.
  Tested in a VM with a WebDAV server: connect, files in Files, read/write/upload, still mounted
  after a reboot, server changes seen within a minute, one sidebar entry, 71 languages.
- Google Drive and Dropbox: the browser opens the provider's own login page. Cancel and closing
  Settings mid-login leave nothing behind (no crash, no rclone left waiting, no half drive).
- New x86 ISO: blank-disk install and first-run setup OK, Cloud Drives page present.
- ARM ISO: rclone, service and page are in it; not booted again (emulation is slow).
- `.upd` re-signed (1.82 GB), verified in MAN. The update flow itself is unchanged and was tested
  earlier today with the previous TEST 4 `.upd`.
- Dock 1.4: it stayed over a window that opened under the pointer; now it steps aside.
Not tested: a real login with an account (no test accounts here). Google's login uses rclone's
shared app key, which Google may retire during 2026; MAN may need its own Google key then.
rclone's own extra questions (rare, provider-specific) are in English.

Earlier on 2026-11-07 (man-installer 3.14.1, desktop 1.3, Settings man8):
- x86 ISO: install to a blank disk, first-run setup (encryption offered), reboot, login,
  /boot and /boot/efi mounted, version 10 TEST 4.
- Update: the published TEST 3 ISO installed, then `lfs/updates/MAN-10-TEST-4.upd` (1.80 GB,
  base 10.10, signed, verified in MAN) applied with MAN Update from a USB stick. Result:
  TEST 4, files kept, fstab repaired, "Your System Was Updated" notice.
- Dock: right-click menu OK; two monitors OK.
- GPL sources: `src/man-desktop/upstream/*.tar.gz` in git.

- ARM ISO (final build): install, first-run setup (encryption offered), /boot and /boot/efi
  mounted, live session starts on the desktop, one click on Install.

Fixed today:
- 3.14.1: /boot and /boot/efi had only 10 s to appear (slow disks: unmounted, encryption hidden).
  New installs drop it; an update repairs the kept fstab.
- Desktop 1.3: the live session opened in the Activities view, so "Install MAN…" needed two clicks.
- ISO boot menu: with the install USB left in, a staged update was skipped (the menu did not read
  the installed grubenv). Fixed and tested on x86. TEST 3's own ISO still has it: a TEST 3 user who
  left the stick in must remove it for the update to apply (it applies on the next start without it).

Published 2026-11-07: pre-release v10-test4 (x86 + ARM ISOs, `.upd`, DUHACH! 1.1.0 x64, all with
.sha256). GitHub turns "!" in file names into ".", so DUHACH! is uploaded as `DUHACH-<ver>-…zip`.
DUHACH! for Windows ARM64 is not uploaded: never run on real Windows ARM.

ARM in the emulator: GNOME needs >2 min to start, so GDM gives up the automatic login
("Session never registered") and shows the login screen; one click logs in. Real ARM is fast.

## TEST 5
Done for TEST 5 (in both images, not in an ISO yet):
- Wallpapers: man-live-config 1.0.6 ships MAN's own set only (MAN + Hintersee, Golden Gate Bridge,
  Western Alps, Yosemite) and removes gnome-backgrounds. A wallpaper whose file is gone is reset to the
  MAN one at login (/usr/libexec/man-wallpaper-check). Tested in an x86 VM.
  Before publishing: confirm the source and licence of hintersee.jpg (unknown; the others are public
  domain or CC0, credited in THIRD-PARTY-NOTICES.md).

1. Security updates: MAN blocks Debian from updating installed packages, so browser, FFmpeg and ClamAV
   only get fixes through a new `.upd`. Needs automatic updates.
2. `yt-dlp` needs its own way to update, or YouTube in Marquee stops working.
3. Marquee sharing: only on trusted networks, ask on new ones (it now starts at every login on any network).
4. Marquee player: subtitles first, then audio track choice, then game controller or remote.

## Smaller
- Marquee's own screens are English only.
- ARM: Windows programs only run if they are ARM64.
- The Dock has no Left/Right position, Trash or Drives (add if wanted).

## Other open items
- The owner's Marquee repo still has the `marquee-share` web page hole; the fix is `src/marquee/share-on-man.patch`.
- MAN Dock is All Rights Reserved (owner's decision) though it builds on GNOME Shell's GPL code.
- Not tested: real hardware, real graphics card, sound in Marquee, real ARM.
- Disk: root-owned leftovers in `~/Documents/MAN3` and `MAN/work` can only go with sudo.
  `~/Documents/MAN` has no git commits, so its scripts and sources are not backed up.
