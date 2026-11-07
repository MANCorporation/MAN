#!/bin/bash
# In the ARM64 builder: MAN 3.14.1 (fstab without the 10 s boot timeout) in the target image.
T=/mnt/target
mkdir -p $T; mountpoint -q $T || mount /dev/vdb $T
mount -t proc proc $T/proc; mount -t sysfs sys $T/sys; mount --rbind /dev $T/dev; mount -t tmpfs tmpfs $T/run
mkdir -p $T/tmp/man-new; cp /mnt/share/debs-man/*_3.14.1_arm64.deb $T/tmp/man-new/
chroot $T env DEBIAN_FRONTEND=noninteractive LC_ALL=C.UTF-8 sh -c 'dpkg -i /tmp/man-new/man-system-tools_*.deb /tmp/man-new/man-installer_*.deb /tmp/man-new/marquee_*.deb 2>&1 | grep -E "^dpkg: error|^Setting up"'; rc=$?
rm -rf $T/tmp/man-new $T/var/lib/clamav/* $T/var/log/clamav/*
chroot $T dpkg -l man-system-tools man-installer marquee 2>&1 | tail -3 | cut -c1-50
cat $T/usr/lib/man/version
umount -R $T/run $T/dev $T/sys $T/proc; sync; umount $T; e2fsck -fn /dev/vdb 2>&1 | tail -1
echo "APPLY3141 DONE rc=$rc"
