#!/bin/bash
# In the ARM64 builder: MAN's own dock (man-desktop-shell 1.2) and Settings man8 in the target image.
T=/mnt/target
mkdir -p $T; mountpoint -q $T || mount /dev/vdb $T
mount -t proc proc $T/proc; mount -t sysfs sys $T/sys; mount --rbind /dev $T/dev; mount -t tmpfs tmpfs $T/run
mkdir -p $T/tmp/man-new; cp /mnt/share/debs-man/man-desktop-shell_1.2_all.deb /mnt/share/debs-man/gnome-control-center_51.1-1+b1man8_arm64.deb $T/tmp/man-new/
chroot $T env DEBIAN_FRONTEND=noninteractive LC_ALL=C.UTF-8 sh -c 'dpkg -i /tmp/man-new/*.deb 2>&1 | grep -E "^dpkg: error|^Setting up (man|gnome)"; gsettings get org.gnome.shell enabled-extensions; gsettings list-keys com.man.desktop.dock | tr "\n" " "'; rc=$?
rm -rf $T/tmp/man-new
chroot $T dpkg -l man-desktop-shell gnome-control-center marquee resources 2>&1 | tail -4 | cut -c1-50
umount -R $T/run $T/dev $T/sys $T/proc; sync; umount $T; e2fsck -fn /dev/vdb 2>&1 | tail -1
echo "DOCK DONE rc=$rc"
