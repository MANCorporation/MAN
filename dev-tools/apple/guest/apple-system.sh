#!/bin/bash
# In the ARM64 builder (dev-tools/arm64/builder.sh with TARGET=lfs/apple/man-gnome-apple.img,
# a copy of lfs/arm64/man-gnome-arm64.img): turn the ARM64 MAN system on /dev/vdb into
# MAN for Apple Silicon Macs.
#   - the Asahi kernel (16K pages) replaces the generic ARM64 kernel
#   - Debian's Asahi packages: firmware loading in the initramfs (asahi-scripts,
#     asahi-fwextract), m1n1, speaker safety and DSP (speakersafetyd, asahi-audio),
#     Touch Bar (tiny-dfr)
#   - Asahi U-Boot built on the PC, at the path update-m1n1 looks for
#   - first start: grow Btrfs into the partition the Asahi installer enlarged
# Files from the PC in /mnt/share/apple: kernel/ (vmlinuz, config, System.map, dtbs/, lib/modules),
# u-boot-nodtb.bin, root/ (MAN's own files for Macs, copied over /).
set -euo pipefail
T=/mnt/target; S=/mnt/share/apple
mkdir -p $T; mountpoint -q $T || mount /dev/vdb $T
mount -t proc proc $T/proc; mount -t sysfs sys $T/sys; mount --rbind /dev $T/dev; mount -t tmpfs tmpfs $T/run
trap 'umount -R $T/run $T/dev $T/sys $T/proc 2>/dev/null || true' EXIT
rm -f $T/etc/resolv.conf; cp -L /etc/resolv.conf $T/etc/resolv.conf
export DEBIAN_FRONTEND=noninteractive LC_ALL=C.UTF-8
inroot() { chroot $T "$@"; }
APT="apt-get -y -q -o Acquire::ForceIPv4=true -o Dpkg::Options::=--force-confold"

echo "== remove the generic ARM64 kernel"
for v in $(ls $T/usr/lib/modules); do
  rm -rf $T/usr/lib/modules/$v $T/usr/lib/linux-image-$v
  rm -f $T/boot/vmlinuz-$v $T/boot/config-$v $T/boot/System.map-$v $T/boot/initrd.img-$v
done

echo "== Asahi kernel"
k=$(ls $S/kernel/lib/modules)
install -m 0644 $S/kernel/vmlinuz $T/boot/vmlinuz-$k
install -m 0644 $S/kernel/config $T/boot/config-$k
install -m 0644 $S/kernel/System.map $T/boot/System.map-$k
mkdir -p $T/usr/lib/modules; cp -a $S/kernel/lib/modules/$k $T/usr/lib/modules/
rm -f $T/usr/lib/modules/$k/build $T/usr/lib/modules/$k/source
mkdir -p $T/usr/lib/linux-image-$k; cp -a $S/kernel/dtbs/. $T/usr/lib/linux-image-$k/
chown -R root:root $T/usr/lib/modules/$k $T/usr/lib/linux-image-$k
inroot depmod $k

echo "== U-Boot"
install -D -m 0644 $S/u-boot-nodtb.bin $T/usr/lib/u-boot/apple_m1/u-boot-nodtb.bin

echo "== Asahi packages"
inroot $APT update >/dev/null
inroot $APT install --no-install-recommends asahi-scripts asahi-fwextract m1n1 \
  speakersafetyd asahi-audio lsp-plugins-lv2 bankstown-lv2 tiny-dfr 2>&1 | grep -E '^E:|upgraded,|^Setting up' || true
for p in asahi-scripts asahi-fwextract m1n1 speakersafetyd asahi-audio tiny-dfr; do
  inroot dpkg-query -W -f='${Package} ${Version} ${Status}\n' $p
done

echo "== newer MAN packages (/mnt/share/apple/debs, optional)"
if ls $S/debs/*.deb >/dev/null 2>&1; then
  rm -rf $T/tmp/man-new; mkdir -p $T/tmp/man-new; cp $S/debs/*.deb $T/tmp/man-new/
  inroot sh -c "$APT install --allow-downgrades /tmp/man-new/*.deb" 2>&1 | grep -E '^E:|upgraded,|^Setting up man|^Setting up marquee' || true
  rm -rf $T/tmp/man-new
  inroot dpkg-query -W -f='${Package} ${Version}\n' man-installer man-system-tools
fi

echo "== MAN files for Macs"
# Files only, owned by root: existing directories (/, /usr, ...) keep their owner and mode.
tar -C $S/root -cf - . | tar -C $T -xf - --no-same-owner --no-overwrite-dir
chown -R root:root $T/usr/lib/systemd/system/man-apple-*.service $T/usr/libexec/man-apple-* \
  $T/usr/bin/man-startup-disk $T/etc/grub.d/44_man_macos $T/usr/share/gnome-shell/extensions/man-mac@man.invalid
# asahi-bless (Asahi, MIT): sets the Mac's start-up disk; built by dev-tools/apple/build-bless.sh.
install -m 755 -o root -g root $S/asahi-bless $T/usr/bin/asahi-bless
inroot systemctl enable man-apple-grow.service man-apple-m1n1.service man-apple-next.service
inroot glib-compile-schemas /usr/share/glib-2.0/schemas
# speakersafetyd is started by its udev rule on Macs whose speakers need it.
inroot systemctl disable speakersafetyd.service 2>/dev/null || true

echo "== checks"
ls $T/usr/lib/aarch64-linux-gnu/dri/ | grep -E 'asahi|apple' || echo "WARNING: no asahi Mesa driver"
ls $T/usr/lib/m1n1/m1n1.bin

echo "== initramfs"
inroot update-initramfs -c -k $k
inroot lsinitramfs /boot/initrd.img-$k > /tmp/initrd-list.txt; grep -cE "asahi|nvme-apple|apple-mailbox" /tmp/initrd-list.txt
inroot apt-get clean
rm -f $T/etc/resolv.conf; : > $T/etc/resolv.conf
echo "APPLE SYSTEM DONE: $k"; df -h $T | tail -1
