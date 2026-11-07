# MAN 10: what is left

## Before publishing TEST 4
Done 2026-11-07:
- x86 ISO: installed to a blank disk, first-run setup, reboot, login, desktop: all OK.
- Dock: right-click menu OK (Dock stays up); two monitors OK (default only on the main one,
  "Show on All Displays" puts it on both, Super takes it away from both).
- `lfs/updates/MAN-10-TEST-4.upd` (1.80 GB, base 10.10): signed, verified inside MAN.
  Notes: `lfs/updates/MAN-10-TEST-4.NOTES.txt`.
- GPL sources: `src/man-desktop/upstream/*.tar.gz` now in git.

Left:
1. ARM ISO install test (see the report of 2026-11-07 for its state).
2. Not tested: updating a real TEST 3 system with the `.upd`.
3. Publishing itself (push, release) needs the owner's OK. Check first that `keys/man-update.key`
   is not included.

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
