#!/bin/bash
# repack-gcc-deb.sh VERSION_SUFFIX: build debs/gnome-control-center_51.0-1<SUFFIX>_amd64.deb
# from the previous MAN package (newest debs/*.deb) with
#   - the binary built by gcc-build-in-vm.sh (out/gnome-control-center),
#   - the Updates panel desktop file, translated from man-settings-po/,
#   - the man-settings catalogs (/usr/share/locale/*/LC_MESSAGES/man-settings.mo).
# Example: ./repack-gcc-deb.sh man4
#
# ARM64: ARCH=arm64 BASE=<Debian's arm64 gnome-control-center .deb> BIN=<binary
# built by lfs/arm64/share/build-apps.sh> ./repack-gcc-deb.sh man6
# (Debian's package of the same Settings version, with MAN's binary in it).
set -euo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
suffix=$1
arch=${ARCH:-amd64}
if [[ $arch == amd64 ]]; then
    base=$(ls -v "$here"/debs/gnome-control-center_51.0-1man*_amd64.deb | tail -n1)
    bin="$here/out/gnome-control-center"
    upstream=51.0-1
else
    base=$BASE bin=$BIN
    upstream=$(dpkg-deb -f "$base" Version | sed 's/^1://')
fi
out="$here/debs/gnome-control-center_${upstream}${suffix}_$arch.deb"
[[ $base != "$out" ]] || { echo "$out exists already" >&2; exit 1; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

dpkg-deb -R "$base" "$work/pkg"
install -m 0755 "$bin" "$work/pkg/usr/bin/gnome-control-center"
mkdir -p "$work/pkg/usr/share/locale"
python3 "$here/man-settings-po/tool.py" build "$work/pkg/usr/share/locale" \
    "$here/gcc-51/panels/man-updates/gnome-man-updates-panel.desktop" \
    "$work/pkg/usr/share/applications/gnome-man-updates-panel.desktop" \
    "$here/gcc-51/panels/man-cloud/gnome-man-cloud-panel.desktop" \
    "$work/pkg/usr/share/applications/gnome-man-cloud-panel.desktop"
chmod 0644 "$work/pkg/usr/share/applications/gnome-man-updates-panel.desktop" \
    "$work/pkg/usr/share/applications/gnome-man-cloud-panel.desktop"
find "$work/pkg/usr/share/locale" -type d -exec chmod 0755 {} + -o -type f -exec chmod 0644 {} +
sed -i "s/^Version: .*/Version: 1:${upstream}${suffix}/" "$work/pkg/DEBIAN/control"
# Built against sid's current AccountsService library.
sed -i -E 's/libaccountsservice0 \([^)]*\)/libaccountsservice1/' "$work/pkg/DEBIAN/control"
(cd "$work/pkg" && find usr -type f -print0 | sort -z | xargs -0 md5sum > DEBIAN/md5sums)
dpkg-deb --root-owner-group -Zxz -b "$work/pkg" "$out" >/dev/null
echo "built $out"
