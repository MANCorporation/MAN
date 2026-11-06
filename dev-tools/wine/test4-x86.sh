#!/bin/bash
# In a VM of a copy of the x86 candidate image (multi-user, no session), folder /mnt/w:
# 3.14.0, MAN's own dock, Settings man8, Marquee, Resources; clean live home; version 10 TEST 4.
set -uo pipefail
cd /mnt/w; export DEBIAN_FRONTEND=noninteractive
H=/home/MANInstaller
echo 'Acquire::ForceIPv4 "true";' > /etc/apt/apt.conf.d/99ipv4-build; apt-get update -q 2>&1 | tail -1
apt-get install -y -q --no-install-recommends -o Dpkg::Options::=--force-confold ./debs/man-system-tools_3.14.0_amd64.deb ./debs/man-installer_3.14.0_amd64.deb ./man-desktop-shell_1.2_all.deb ./gnome-control-center_51.0-1man8_amd64.deb 2>&1 | grep -E '^E:|upgraded,' | cut -c1-120
apt-get install -y -q --no-install-recommends -o Dpkg::Options::=--force-confold -o Dir::Etc::Preferences=/dev/null -o Dir::Etc::PreferencesParts=/dev/null resources yt-dlp ffmpeg libavcodec63 ./debs/marquee_3.14.0_amd64.deb 2>&1 | grep -E '^E:|upgraded,' | cut -c1-120
apt-get purge -y -q gnome-system-monitor 2>&1 | tail -1 | cut -c1-80
apt-get clean; rm -f /etc/apt/apt.conf.d/99ipv4-build
# The live user's home as it was before an old session filled it.
rm -rf $H; cp -a /mnt/w/cleanhome/MANInstaller $H; rm -f $H/.face $H/.face.icon
install -d $H/.config/gtk-4.0; install -m 0644 /etc/skel/.config/gtk-4.0/gtk.css $H/.config/gtk-4.0/gtk.css
chown -hR 1001:1001 $H; chmod 0750 $H 2>/dev/null
rm -rf /var/lib/clamav/* /var/log/clamav/*; find /var/log -type f -name '*.gz' -delete; journalctl --vacuum-size=1M >/dev/null 2>&1
man-set-version "10 TEST 4" "$(cat /usr/lib/man/version-base)"
cat /usr/lib/man/version /usr/lib/man/version-base; grep ^PRETTY /usr/lib/os-release
dpkg -l man-system-tools man-installer man-desktop-shell gnome-control-center marquee resources ffmpeg yt-dlp wine gnome-system-monitor 2>&1 | tail -10 | cut -c1-48
ldd /usr/bin/gnome-shell | grep -c 'not found'
find $H -type f | wc -l; ls -A $H | tr '\n' ' '; echo
ls /var/lib/clamav | wc -l; df -BM --output=used / | tail -1
sync; echo RECIPE DONE
