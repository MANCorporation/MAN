# MAN 10: what is left

## Before publishing TEST 4
Done 2026-11-07 (final build: man-installer 3.14.1, desktop 1.3, Settings man8):
- x86 ISO: install to a blank disk, first-run setup (encryption offered), reboot, login,
  /boot and /boot/efi mounted, version 10 TEST 4.
- Update: the published TEST 3 ISO installed, then `lfs/updates/MAN-10-TEST-4.upd` (1.80 GB,
  base 10.10, signed, verified in MAN) applied with MAN Update from a USB stick. Result:
  TEST 4, files kept, fstab repaired, "Your System Was Updated" notice.
- Dock: right-click menu OK; two monitors OK.
- GPL sources: `src/man-desktop/upstream/*.tar.gz` in git.

Fixed today:
- 3.14.1: /boot and /boot/efi had only 10 s to appear (slow disks: unmounted, encryption hidden).
  New installs drop it; an update repairs the kept fstab.
- Desktop 1.3: the live session opened in the Activities view, so "Install MAN…" needed two clicks.
- ISO boot menu: with the install USB left in, a staged update was skipped (the menu did not read
  the installed grubenv). Fixed and tested on x86. TEST 3's own ISO still has it: a TEST 3 user who
  left the stick in must remove it for the update to apply (it applies on the next start without it).

Left:
1. ARM ISO install with the final build (running 2026-11-07 afternoon).
2. Publishing itself (push, release) needs the owner's OK. Check first that `keys/man-update.key`
   is not included.

ARM in the emulator: GNOME needs >2 min to start, so GDM gives up the automatic login
("Session never registered") and shows the login screen; one click logs in. Real ARM is fast.

## TEST 5
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
