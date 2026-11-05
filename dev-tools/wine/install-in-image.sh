#!/bin/bash
# Run as root inside a VM of the MAN image (x86-64), with a folder holding the new
# packages as $1: Wine, ClamAV and MAN's Windows Program check; also removes what
# the image carried without using it, to stay under GitHub's 2 GiB per file.
set -euo pipefail
D=${1:?folder with man-*.deb, man-wine-dialogs*.deb}
# ARM64: this script with ARCH=arm64: no slimming (that image never had the leftovers)
# and no man-wine-dialogs. Debian's Wine there only runs ARM64 programs.
arch=${ARCH:-amd64}
export DEBIAN_FRONTEND=noninteractive
echo 'Acquire::ForceIPv4 "true";' > /etc/apt/apt.conf.d/99ipv4-build
apt-get update -q
if [ "$arch" = amd64 ]; then
    echo "== slimming"
    apt-get install -y -q --no-install-recommends -o Dpkg::Options::=--force-confold "$D"/man-live-config_*_all.deb
    apt-get purge -y -q linux-image-amd64 'linux-image-6.12*'
    rm -rf /usr/lib/modules/7.2.6+deb14-amd64 /usr/lib/modules/6.12.107+deb13-amd64
    rm -f /qemu_gnome_boot*.log /test-d.img /test-ext4.img
fi
# Debian's logo as every new user's picture.
rm -f /etc/skel/.face /etc/skel/.face.icon /home/*/.face /home/*/.face.icon
echo "== Wine and ClamAV"
apt-get install -y -q --no-install-recommends wine wine64 clamav clamav-freshclam fonts-liberation libodbc2
dialogs=""
[ "$arch" = amd64 ] && dialogs=$(ls "$D"/man-wine-dialogs_*_amd64.deb)
apt-get install -y -q --no-install-recommends -o Dpkg::Options::=--force-confold \
    $dialogs "$D"/man-system-tools_*_"$arch".deb "$D"/man-installer_*_"$arch".deb \
    "$D"/man-desktop-shell_*_all.deb "$D"/gnome-control-center_*man*_"$arch".deb
systemctl enable clamav-freshclam.service
# Popups like macOS for the live user too (new users get it from /etc/skel).
if [ -d /home/MANInstaller ]; then
    install -d -o 1001 -g 1001 /home/MANInstaller/.config/gtk-4.0
    install -o 1001 -g 1001 -m 0644 /etc/skel/.config/gtk-4.0/gtk.css /home/MANInstaller/.config/gtk-4.0/gtk.css
fi
# No malware database in the image: it downloads when the computer has a network
# (90-man-freshclam), and would not fit in the ISO.
systemctl stop clamav-freshclam.service 2>/dev/null || true
rm -rf /var/lib/clamav/* /var/log/clamav/*
apt-get clean
rm -f /etc/apt/apt.conf.d/99ipv4-build
ls -la /var/lib/clamav
dpkg -l wine libwine clamav man-system-tools man-installer man-live-config man-desktop-shell gnome-control-center | tail -8 | cut -c1-60
df -BM --output=used / | tail -1
