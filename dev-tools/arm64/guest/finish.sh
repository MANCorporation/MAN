#!/bin/bash
# Inside the ARM64 builder, after mkroot.sh: turn the Debian sid arm64 system
# on /dev/vdb into MAN (kernel, MAN packages, MAN's files, accounts).
#   VERSION="10 TEST 4" BASE=10.10 bash /mnt/share/finish.sh
set -euo pipefail
T=/mnt/target; S=/mnt/share
VERSION=${VERSION:-10 TEST 4}; BASE=${BASE:-10.10}
mountpoint -q $T || mount /dev/vdb $T
mount -t proc proc $T/proc; mount -t sysfs sys $T/sys; mount --rbind /dev $T/dev; mount -t tmpfs tmpfs $T/run
trap 'umount -R $T/run $T/dev $T/sys $T/proc 2>/dev/null || true' EXIT
rm -f $T/etc/resolv.conf; cp -L /etc/resolv.conf $T/etc/resolv.conf
export DEBIAN_FRONTEND=noninteractive LC_ALL=C.UTF-8
inroot() { chroot $T "$@"; }

echo "== kernel"
install -D -m 0755 $S/busybox-arm64 $T/usr/lib/man-initramfs/busybox
k=$(ls $S/kernel/lib/modules); 
install -m 0644 $S/kernel/vmlinuz $T/boot/vmlinuz-$k
install -m 0644 $S/kernel/config $T/boot/config-$k
install -m 0644 $S/kernel/System.map $T/boot/System.map-$k
rm -rf $T/usr/lib/modules/$k; mkdir -p $T/usr/lib/modules; cp -a $S/kernel/lib/modules/$k $T/usr/lib/modules/
rm -f $T/usr/lib/modules/$k/build $T/usr/lib/modules/$k/source
chown -R root:root $T/usr/lib/modules/$k
mkdir -p $T/usr/lib/linux-image-$k; cp -a $S/kernel/dtbs/. $T/usr/lib/linux-image-$k/
chown -R root:root $T/usr/lib/linux-image-$k
inroot depmod $k

echo "== MAN packages"
rm -rf $T/tmp/man-debs; mkdir -p $T/tmp/man-debs; cp $S/debs-man/*.deb $T/tmp/man-debs/
inroot apt-get -q -o Acquire::ForceIPv4=true update >/dev/null
inroot bash -c "apt-get install -y -q -o Acquire::ForceIPv4=true -o Dpkg::Options::=--force-confold --allow-downgrades /tmp/man-debs/*.deb"
rm -rf $T/tmp/man-debs

echo "== MAN files and accounts"
tar -C $T --numeric-owner -xpf $S/manfiles.tar
while IFS=: read -r name rest; do grep -q "^$name:" $T/etc/passwd || echo "$name:$rest" >> $T/etc/passwd; done < $S/users.passwd
while IFS=: read -r name rest; do grep -q "^$name:" $T/etc/shadow || echo "$name:$rest" >> $T/etc/shadow; done < $S/users.shadow
while IFS=: read -r name rest; do grep -q "^$name:" $T/etc/group || echo "$name:$rest" >> $T/etc/group; done < $S/users.group
inroot grpconv
while read -r g members; do for m in ${members//,/ }; do inroot getent group "$g" >/dev/null && inroot gpasswd -a "$m" "$g" >/dev/null || true; done; done < $S/users.members
tar -C $T --numeric-owner -xpf $S/home.tar
echo man > $T/etc/hostname

echo "== version and caches"
inroot man-set-version "$VERSION" "$BASE"
inroot glib-compile-schemas /usr/share/glib-2.0/schemas
inroot dconf update || true
[ -x $T/usr/share/gdm/generate-config ] && inroot /usr/share/gdm/generate-config || true
inroot update-desktop-database -q /usr/share/applications || true
inroot gtk-update-icon-cache -q -f /usr/share/icons/hicolor || true
inroot update-mime-database /usr/share/mime || true
inroot locale-gen >/dev/null 2>&1 || true
inroot update-initramfs -c -k $k
inroot apt-get clean
rm -f $T/etc/resolv.conf; ln -s ../run/systemd/resolve/stub-resolv.conf $T/etc/resolv.conf 2>/dev/null || true
: > $T/etc/machine-id
echo "FINISH DONE: MAN $VERSION arm64"; df -h $T | tail -1
