#!/bin/bash
# In the ARM64 builder: stamp the target image as MAN 10 TEST 5 (made after 10.10).
T=/mnt/target
mkdir -p $T; mountpoint -q $T || mount /dev/vdb $T
mount -t proc proc $T/proc; mount --rbind /dev $T/dev
chroot $T man-set-version "10 TEST 5" 10.10; rc=$?
cat $T/usr/lib/man/version $T/usr/lib/man/version-base; grep '^VERSION=' $T/etc/os-release
umount -R $T/dev $T/proc; sync; umount $T; e2fsck -fn /dev/vdb 2>&1 | tail -1
echo "TEST5 DONE rc=$rc"
