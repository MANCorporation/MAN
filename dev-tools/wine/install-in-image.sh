#!/bin/bash
# Run as root inside a VM of the MAN image (x86-64), with a folder holding the new
# packages as $1: Wine, ClamAV and MAN's Windows Program check; also removes what
# the image carried without using it, to stay under GitHub's 2 GiB per file.
set -euo pipefail
D=${1:?folder with man-*.deb, man-wine-dialogs*.deb}
export DEBIAN_FRONTEND=noninteractive
echo 'Acquire::ForceIPv4 "true";' > /etc/apt/apt.conf.d/99ipv4-build
apt-get update -q
echo "== slimming"
apt-get install -y -q --no-install-recommends -o Dpkg::Options::=--force-confold "$D"/man-live-config_*_all.deb
apt-get purge -y -q linux-image-amd64 'linux-image-6.12*'
rm -rf /usr/lib/modules/7.2.6+deb14-amd64 /usr/lib/modules/6.12.107+deb13-amd64
rm -f /qemu_gnome_boot*.log /test-d.img /test-ext4.img
# Debian's logo as every new user's picture.
rm -f /etc/skel/.face /etc/skel/.face.icon /home/*/.face /home/*/.face.icon
echo "== Wine and ClamAV"
apt-get install -y -q --no-install-recommends wine wine64 clamav clamav-freshclam fonts-liberation libodbc2
apt-get install -y -q --no-install-recommends -o Dpkg::Options::=--force-confold \
    "$D"/man-wine-dialogs_*_amd64.deb "$D"/man-system-tools_*_amd64.deb "$D"/man-installer_*_amd64.deb
systemctl enable clamav-freshclam.service
echo "== malware database"
systemctl stop clamav-freshclam.service 2>/dev/null || true
freshclam --quiet || echo "freshclam failed: the database downloads on first use"
apt-get clean
rm -f /etc/apt/apt.conf.d/99ipv4-build /var/log/clamav/*
ls -la /var/lib/clamav
dpkg -l wine libwine clamav man-system-tools man-installer man-live-config man-wine-dialogs | tail -7 | cut -c1-60
df -BM --output=used / | tail -1
