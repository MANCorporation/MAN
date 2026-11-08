# Sound on Macs with the Cirrus Logic CS8409 codec

The kernel's `snd-hda-codec-cs8409` supports Dell laptops with this codec, not Apple's
(subsystem 0x106b…, with CS42L83 headphone chip and TI/Maxim/ADI amplifiers). On an iMac 2017
the speakers are listed but no sound comes out. MAN replaces the module with this driver:

- Driver: https://github.com/davidjo/snd_hda_macbookpro, commit 89b22ff (2026-09-06), GPL-2.0
  (`LICENSE`). Supports MacBook Pro 14,x/15,x/16,x and iMac 18,1 / 18,2 / 18,3 / 19,1.
- iMac 2017 fixes: https://github.com/riqgonzalo/imac-2017-linux-audio-cs8409, commit 80675a4,
  GPL-2.0, `imac-2017.patch` (already applied to the files here): headphone jack sense not
  inverted on iMacs (the inverted sense made the codec think headphones were plugged in and muted
  the speakers) and a `force_speakers` module option.

Files: `patch_cirrus/` (Apple codec code, included into cs8409.c), `patch_cs8409.[ch].diff`
(applied to the kernel's `sound/hda/codecs/cirrus/cs8409.[ch]`), `makefiles/` (build only this
module). Build: `gnome51-fix/11-mac-audio.sh` (in the LFS chroot). The result replaces
`/usr/lib/modules/<kernel>/kernel/sound/hda/codecs/cirrus/snd-hda-codec-cs8409.ko` in the image
(then `depmod`); rebuild it whenever the kernel is rebuilt.

Known limits (from the driver's README): microphone input is incomplete and quiet; output is
2 or 4 channels at 44.1 kHz; the amplifiers stay powered (suspend untested).
