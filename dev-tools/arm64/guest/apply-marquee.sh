#!/bin/bash
# In the ARM64 builder: 3.14.0, Marquee, Resources and the desktop package in the target image.
T=/mnt/target
mountpoint -q $T || mount /dev/vdb $T
mount -t proc proc $T/proc; mount -t sysfs sys $T/sys; mount --rbind /dev $T/dev; mount -t tmpfs tmpfs $T/run
mkdir -p $T/tmp/man-new; cd /mnt/share/debs-man
cp man-system-tools_3.14.0_arm64.deb man-installer_3.14.0_arm64.deb marquee_3.14.0_arm64.deb man-desktop-shell_1.1_all.deb $T/tmp/man-new/
rm -f $T/etc/resolv.conf; cp -L /etc/resolv.conf $T/etc/resolv.conf
chroot $T env DEBIAN_FRONTEND=noninteractive LC_ALL=C.UTF-8 sh -c 'apt-get -o Acquire::ForceIPv4=true update -q >/dev/null
apt-get -o Acquire::ForceIPv4=true install -y -q --no-install-recommends -o Dpkg::Options::=--force-confold -o Dir::Etc::Preferences=/dev/null -o Dir::Etc::PreferencesParts=/dev/null /tmp/man-new/man-system-tools_*.deb /tmp/man-new/man-installer_*.deb /tmp/man-new/man-desktop-shell_*.deb resources yt-dlp ffmpeg libavcodec63 /tmp/man-new/marquee_*.deb 2>&1 | grep -E "^E:|upgraded,"
apt-get purge -y -q gnome-system-monitor 2>&1 | tail -1 | cut -c1-80
apt-get clean'; rc=$?
rm -rf $T/tmp/man-new
rm -rf $T/var/lib/clamav/* $T/var/log/clamav/*
rm -f $T/etc/resolv.conf; : > $T/etc/resolv.conf
chroot $T dpkg -l man-system-tools man-installer man-desktop-shell marquee resources ffmpeg yt-dlp gnome-system-monitor 2>&1 | tail -8 | cut -c1-55
grep -c background-opacity=0.55 $T/usr/share/glib-2.0/schemas/20_man-desktop-shell.gschema.override
df -BM --output=used $T | tail -1
umount -R $T/run $T/dev $T/sys $T/proc; sync; umount $T; e2fsck -fn /dev/vdb 2>&1 | tail -1
echo "MARQUEE DONE rc=$rc"
