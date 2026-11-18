#!/bin/bash
# Builds the man-grub package: MAN's GRUB (Debian's GRUB + man-bootmenu.patch)
# for the boot menu, its theme, and a grub-install that uses it.
#   dev-tools/grub-src/make-deb.sh amd64|arm64 [OUT_DIR]
# Needs the GRUB build first: build.sh efi (amd64) or build.sh arm64.
# Output: OUT_DIR (default man-installer/target/debs)/man-grub_<ver>_<arch>.deb
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
p=$(cd "$here/../.." && pwd)
arch=${1:?amd64 or arm64}
out=${2:-$p/man-installer/target/debs}
data=$p/man-installer/data/grub
case $arch in
    # (The PC image's package list says GRUB 2.12, its files are 2.14-3.)
    amd64) stage=$here/build2143/stage; plats="x86_64-efi"; base=2.14-3; grub_bin=grub-efi-amd64-bin ;;
    arm64) stage=$here/build214/stage; plats="arm64-efi"; base=2.14-4; grub_bin=grub-efi-arm64-bin ;;
    *) echo "amd64 or arm64" >&2; exit 2 ;;
esac
# Bump the last number for every change to the patch or the theme.
version=$base+man1
r=$(mktemp -d)
trap 'rm -r "$r"' EXIT

for pl in $plats; do
    [ -f "$stage/usr/lib/grub/$pl/kernel.img" ] || { echo "No GRUB build for $pl in $stage" >&2; exit 1; }
    mkdir -p "$r/usr/lib/man/grub"
    cp -r "$stage/usr/lib/grub/$pl" "$r/usr/lib/man/grub/$pl"
done
echo "$version" > "$r/usr/lib/man/grub/version"
install -D -m 755 "$data/man-grub/grub-install" "$r/usr/sbin/grub-install"
install -D -m 755 "$data/man-grub/man-grub-refresh" "$r/usr/libexec/man/man-grub-refresh"
mkdir -p "$r/usr/share/grub/themes"
cp -r "$data/theme/man" "$r/usr/share/grub/themes/man"
d=$r/usr/share/doc/man-grub
mkdir -p "$d"
cat > "$d/copyright" <<EOF
man-grub: GRUB with MAN's boot menu additions.

GRUB: Copyright (C) Free Software Foundation, Inc. and the GRUB contributors.
License: GPL-3.0-or-later (see /usr/share/common-licenses/GPL-3).
Built from Debian's grub2 $base source package and MAN's patch
man-bootmenu.patch (included here, same license). The exact sources are
kept in the MAN repository: dev-tools/grub-src/ (grub2_${base}.dsc, the
.orig and .debian tarballs, man-bootmenu.patch, build.sh).

Theme (/usr/share/grub/themes/man): icons drawn for MAN, Copyright MAN
Developers. Fonts made from Adwaita Sans: Copyright The Adwaita Sans
Authors, SIL Open Font License 1.1.
Windows and macOS names in the menu only say which system an entry starts;
they are trademarks of their owners.
EOF
cp "$here/man-bootmenu.patch" "$d/man-bootmenu.patch"
chmod -R u+rwX,go+rX,go-w "$r"

mkdir -p "$r/DEBIAN"
size=$(du -sk "$r" | cut -f1)
cat > "$r/DEBIAN/control" <<EOF
Package: man-grub
Version: $version
Architecture: $arch
Maintainer: MAN Developers <dev@man.local>
Section: admin
Priority: optional
Installed-Size: $size
Depends: grub2-common, grub-common, $grub_bin
Description: MAN's boot menu (GRUB with a start-up disk picker)
 MAN starts straight away. Holding Alt (Option) at power-on shows a row of
 large icons: MAN, Windows, macOS, USB drives and MAN Recovery. Ctrl+Enter
 makes a system the one that starts by default, Shift+Enter on MAN shows
 the advanced options (other kernels, snapshots, firmware settings).
EOF
cat > "$r/DEBIAN/preinst" <<'EOF'
#!/bin/sh
set -e
if [ "$1" = install ] || [ "$1" = upgrade ]; then
    dpkg-divert --package man-grub --rename --quiet \
        --divert /usr/sbin/grub-install.debian --add /usr/sbin/grub-install
fi
EOF
cat > "$r/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
if [ "$1" = configure ]; then
    # An installed MAN: put the new GRUB on the EFI partition and rewrite the menu.
    if [ -f /boot/grub/grub.cfg ] && mountpoint -q /boot/efi 2>/dev/null; then
        /usr/libexec/man/man-grub-refresh || echo "man-grub: the boot loader was not refreshed" >&2
        update-grub || true
    fi
fi
EOF
cat > "$r/DEBIAN/postrm" <<'EOF'
#!/bin/sh
set -e
if [ "$1" = remove ] || [ "$1" = purge ] || [ "$1" = abort-install ]; then
    dpkg-divert --package man-grub --rename --quiet --remove /usr/sbin/grub-install || true
fi
EOF
chmod 755 "$r/DEBIAN/preinst" "$r/DEBIAN/postinst" "$r/DEBIAN/postrm"
mkdir -p "$out"
dpkg-deb --root-owner-group -Zxz -b "$r" "$out/man-grub_${version}_$arch.deb" >/dev/null
ls -la "$out/man-grub_${version}_$arch.deb"
