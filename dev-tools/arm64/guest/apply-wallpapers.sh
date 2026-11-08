#!/bin/bash
# In the ARM64 builder: man-live-config 1.0.6 (MAN wallpapers only, gnome-backgrounds removed).
T=/mnt/target
mkdir -p $T; mountpoint -q $T || mount /dev/vdb $T
mount -t proc proc $T/proc; mount -t sysfs sys $T/sys; mount --rbind /dev $T/dev; mount -t tmpfs tmpfs $T/run
mkdir -p $T/tmp/man-new; cp /mnt/share/debs-man/man-live-config_1.0.6_all.deb $T/tmp/man-new/
chroot $T env DEBIAN_FRONTEND=noninteractive LC_ALL=C.UTF-8 sh -c 'apt-get install -y -q --no-install-recommends -o Dir::Etc::Preferences=/dev/null -o Dir::Etc::PreferencesParts=/dev/null /tmp/man-new/man-live-config_1.0.6_all.deb 2>&1 | grep -E "^E:|upgraded,|^  gnome"
apt-get clean'; rc=$?
rm -rf $T/tmp/man-new
ls $T/usr/share/backgrounds $T/usr/share/backgrounds/man $T/usr/share/gnome-background-properties | tr '\n' ' '; echo
ls $T/var/cache/man-oobe/thumbnails | wc -l
umount -R $T/run $T/dev $T/sys $T/proc; sync; umount $T; e2fsck -fn /dev/vdb 2>&1 | tail -1
echo "WALLPAPERS DONE rc=$rc"
