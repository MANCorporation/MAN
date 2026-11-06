# MAN 10: what is left

## Before publishing TEST 4
1. Full install test: install each ISO to a disk, then finish first-run setup.
2. Build the TEST 4 `.upd` from `lfs/.vmtest/test4.img` (x86), so TEST 3 systems can update.
3. GPL source with the release: Blur and Window Tiling (`src/man-desktop/upstream/`, not in git),
   the patched Settings and the patched Wine files.
4. Dock: test the right-click menu on an icon (does the Dock stay up while it is open?) and two monitors.
5. Publishing itself (push, release) needs the owner's OK. Check first that `keys/man-update.key` is not included.

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
