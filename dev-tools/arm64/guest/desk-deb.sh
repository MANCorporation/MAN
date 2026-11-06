T=/mnt/target
mount /dev/vdb $T; mount -t proc proc $T/proc; mount --rbind /dev $T/dev; mount -t sysfs sys $T/sys
cp /mnt/share/debs-man/man-desktop-shell_1.0_all.deb $T/tmp/
chroot $T dpkg -i /tmp/man-desktop-shell_1.0_all.deb 2>&1 | tail -1 | cut -c1-60
rm -f $T/tmp/man-desktop-shell_1.0_all.deb
grep -c "enabled-extensions=\['man-look@man.invalid', 'man-dock@man.invalid', 'man-blur@man.invalid'\]" $T/usr/share/glib-2.0/schemas/20_man-desktop-shell.gschema.override
chroot $T gsettings get org.gnome.shell enabled-extensions
ls $T/var/lib/clamav | wc -l; : > /dev/null; ls -la $T/etc/resolv.conf | cut -c1-40
umount -R $T/dev $T/sys $T/proc; sync; umount $T; e2fsck -fn /dev/vdb | tail -1
echo DESKDEB DONE
