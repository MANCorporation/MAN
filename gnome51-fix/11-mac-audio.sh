#!/bin/bash
# MAN kernel: sound on Macs with a Cirrus Logic CS8409 codec (iMac 2017 = iMac18,1/18,2/18,3,
# iMac 2019, MacBook Pro 2016-2019 without T2 audio). The kernel's own snd-hda-codec-cs8409 does
# not know Apple's codecs and amplifiers: the speakers show up but stay silent. This builds the
# module from davidjo/snd_hda_macbookpro with the iMac 2017 fixes (src/mac-audio/README.md),
# against the MAN kernel's own sound/hda sources.
# Run INSIDE the LFS chroot, with src/mac-audio copied to /sources/man-mac-audio-src:
#   bash 11-mac-audio.sh   ->  /sources/man-mac-audio/snd-hda-codec-cs8409.ko
set -eo pipefail
export PATH=/usr/local/bin:/usr/bin:/usr/sbin:/tools/bin
K=/sources/linux-build-gnome51; SRC=/sources/man-mac-audio-src; W=/sources/man-mac-audio
rm -rf $W; mkdir -p $W/hda
# Only the sources of sound/hda (the build tree also holds objects).
(cd $K/sound/hda && find . \( -name '*.c' -o -name '*.h' -o -name Makefile -o -name Kconfig \) \
    -exec cp --parents {} $W/hda/ \;)
H=$W/hda
cp $SRC/makefiles/Makefile $H/Makefile
cp $SRC/makefiles/Makefile_common $H/common/Makefile
cp $SRC/makefiles/Makefile_codecs $H/codecs/Makefile
cp $SRC/makefiles/Makefile_cirrus $H/codecs/cirrus/Makefile
for f in cirrus_apple.h patch_cirrus_boot84.h patch_cirrus_new84.h patch_cirrus_real84.h \
         patch_cirrus_hda_generic_copy.h patch_cirrus_real84_i2c.h; do
    cp $SRC/patch_cirrus/$f $H/codecs/cirrus/
done
(cd $H && patch -p1 --no-backup-if-mismatch < $SRC/patch_cs8409.c.diff && patch -p1 --no-backup-if-mismatch < $SRC/patch_cs8409.h.diff)
make -C $K M=$H CFLAGS_MODULE="-DAPPLE_PINSENSE_FIXUP -DAPPLE_CODECS -DCONFIG_SND_HDA_RECONFIG=1 -Wno-unused-variable -Wno-unused-function" modules
strip --strip-debug -o $W/snd-hda-codec-cs8409.ko $H/codecs/cirrus/snd-hda-codec-cs8409.ko
modinfo $W/snd-hda-codec-cs8409.ko | grep -E '^(vermagic|parm|depends)'
echo "MAC AUDIO BUILT"
