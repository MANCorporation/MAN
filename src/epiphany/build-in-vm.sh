#!/bin/sh
# Run as root inside the build VM (src shared at /mnt/src): builds PCD debs into /mnt/src/epiphany/debs.
set -e
rm -rf /w/ep && mkdir -p /w/ep && cd /w/ep
cp /mnt/src/epiphany/epiphany-browser_51.1* .
dpkg-source -x epiphany-browser_51.1-1.dsc src >/dev/null
cd src
cp /mnt/src/epiphany/man-pcd-branding.patch debian/patches/
echo man-pcd-branding.patch >> debian/patches/series
sed -i '/usr\/share\/help/d' debian/epiphany-browser-data.install
{ printf 'epiphany-browser (51.1-1man1) unstable; urgency=medium\n\n  * MAN: rebrand as PCD (name, icon, About, menus, metadata, no help manual).\n\n -- MAN Developers <dev@man.local>  %s\n\n' "$(date -R)"; cat debian/changelog; } > debian/changelog.new
mv debian/changelog.new debian/changelog
DEB_BUILD_OPTIONS="nocheck parallel=4" dpkg-buildpackage -b -uc -us > /w/ep/build.log 2>&1
mkdir -p /mnt/src/epiphany/debs
cp /w/ep/*.deb /mnt/src/epiphany/debs/
