#!/bin/bash
# In the ARM64 builder: MAN's desktop (Dock, Blur, Window Tiling, dialogs) and 3.13.0 in the target image.
T=/mnt/target
mountpoint -q $T || mount /dev/vdb $T
mount -t proc proc $T/proc; mount -t sysfs sys $T/sys; mount --rbind /dev $T/dev; mount -t tmpfs tmpfs $T/run
mkdir -p $T/tmp/man-new; cd /mnt/share/debs-man
cp man-system-tools_3.13.0_arm64.deb man-installer_3.13.0_arm64.deb man-desktop-shell_1.0_all.deb gnome-control-center_51.1-1+b1man7_arm64.deb $T/tmp/man-new/
rm -f $T/etc/resolv.conf; cp -L /etc/resolv.conf $T/etc/resolv.conf
chroot $T env DEBIAN_FRONTEND=noninteractive LC_ALL=C.UTF-8 sh -c 'apt-get -o Acquire::ForceIPv4=true update -q >/dev/null; apt-get -o Acquire::ForceIPv4=true install -y -q --no-install-recommends -o Dpkg::Options::=--force-confold /tmp/man-new/*.deb 2>&1 | tail -2; apt-get clean; rm -rf /var/lib/apt/lists/*'; rc=$?
rm -rf $T/tmp/man-new
if [ -d $T/home/MANInstaller ]; then
  install -d -o 1001 -g 1001 $T/home/MANInstaller/.config/gtk-4.0
  install -o 1001 -g 1001 -m 0644 $T/etc/skel/.config/gtk-4.0/gtk.css $T/home/MANInstaller/.config/gtk-4.0/gtk.css
fi
rm -rf $T/var/lib/clamav/* $T/var/log/clamav/*
rm -f $T/etc/resolv.conf; : > $T/etc/resolv.conf
chroot $T dpkg -l man-system-tools man-installer man-desktop-shell gnome-control-center gir1.2-dbusmenu-glib-0.4 | tail -5 | cut -c1-55
chroot $T gsettings get org.gnome.shell enabled-extensions 2>/dev/null
ls $T/home/MANInstaller/Downloads 2>/dev/null | wc -l; cat $T/usr/lib/man/version
umount -R $T/run $T/dev $T/sys $T/proc; sync; umount $T; e2fsck -fn /dev/vdb 2>&1 | tail -1
echo DESK DONE rc=$rc
