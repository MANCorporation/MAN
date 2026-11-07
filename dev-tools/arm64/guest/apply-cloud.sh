#!/bin/bash
# In the ARM64 builder: Cloud Drives (rclone, MAN 3.15.0, Settings man9) and desktop 1.4 in the target image.
T=/mnt/target
mkdir -p $T; mountpoint -q $T || mount /dev/vdb $T
mount -t proc proc $T/proc; mount -t sysfs sys $T/sys; mount --rbind /dev $T/dev; mount -t tmpfs tmpfs $T/run
mkdir -p $T/tmp/man-new; cd /mnt/share/debs-man
cp man-system-tools_3.15.0_arm64.deb man-installer_3.15.0_arm64.deb marquee_3.15.0_arm64.deb man-desktop-shell_1.4_all.deb gnome-control-center_51.1-1+b1man9_arm64.deb $T/tmp/man-new/
rm -f $T/etc/resolv.conf; cp -L /etc/resolv.conf $T/etc/resolv.conf
chroot $T env DEBIAN_FRONTEND=noninteractive LC_ALL=C.UTF-8 sh -c 'apt-get -o Acquire::ForceIPv4=true update -q >/dev/null
apt-get -o Acquire::ForceIPv4=true install -y -q --no-install-recommends -o Dpkg::Options::=--force-confold -o Dir::Etc::Preferences=/dev/null -o Dir::Etc::PreferencesParts=/dev/null rclone /tmp/man-new/*.deb 2>&1 | grep -E "^E:|upgraded,"
apt-get clean'; rc=$?
rm -rf $T/tmp/man-new $T/var/lib/clamav/* $T/var/log/clamav/*
rm -f $T/etc/resolv.conf; : > $T/etc/resolv.conf
chroot $T dpkg -l rclone man-system-tools man-installer marquee man-desktop-shell gnome-control-center 2>&1 | tail -6 | cut -c1-55
ls $T/usr/lib/systemd/user/man-cloud@.service $T/usr/share/applications/gnome-man-cloud-panel.desktop
umount -R $T/run $T/dev $T/sys $T/proc; sync; umount $T; e2fsck -fn /dev/vdb 2>&1 | tail -1
echo "CLOUD DONE rc=$rc"
