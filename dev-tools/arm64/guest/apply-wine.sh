#!/bin/bash
# Inside the ARM64 builder: dev-tools/wine/install-in-image.sh in the target image.
T=/mnt/target
mountpoint -q $T || mount /dev/vdb $T
mount -t proc proc $T/proc; mount -t sysfs sys $T/sys; mount --rbind /dev $T/dev; mount -t tmpfs tmpfs $T/run
mkdir -p $T/tmp/man-new; cp /mnt/share/debs-man/man-system-tools_3.12.0_arm64.deb /mnt/share/debs-man/man-installer_3.12.0_arm64.deb $T/tmp/man-new/
cp /mnt/share/install-in-image.sh $T/tmp/man-new/
rm -f $T/etc/resolv.conf; cp -L /etc/resolv.conf $T/etc/resolv.conf
chroot $T env ARCH=arm64 LC_ALL=C.UTF-8 bash /tmp/man-new/install-in-image.sh /tmp/man-new; rc=$?
rm -rf $T/tmp/man-new
rm -f $T/etc/resolv.conf; : > $T/etc/resolv.conf
umount -R $T/run $T/dev $T/sys $T/proc; sync
chroot $T true 2>/dev/null
ls $T/var/lib/clamav | wc -l
grep -c "Spustit tento program" $T/usr/share/locale/cs/LC_MESSAGES/man-installer.mo
cat $T/usr/lib/man/version; grep ^PRETTY_NAME $T/usr/lib/os-release
du -sh $T/usr/lib/aarch64-linux-gnu/wine 2>/dev/null
df -BM --output=used $T | tail -1
umount $T; e2fsck -fn /dev/vdb 2>&1 | tail -1
echo "APPLY DONE rc=$rc"
