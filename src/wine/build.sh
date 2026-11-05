#!/usr/bin/env bash
# build.sh: Debian's Wine source + MAN's patch → the patched comdlg32 and shell32
# (x86_64, i386) → debs/man-wine-dialogs_<version>_amd64.deb. Nothing is installed.
# Debian's source lacks files generated at build time: only the generators these
# DLLs need are run, the text-shaping tables of other DLLs are left empty.
set -euo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd); P=$here/../..; H=$P/tools/hostbin/root
W=${WORK:-/tmp/man-wine-build}; ver=10.0~repack-12; deb_ver=10.0-12man1
export PATH=$H/alias:$P/tools/rsvg:$H/usr/bin:$P/tools/mingw/root/usr/bin:$PATH BISON_PKGDATADIR=$H/usr/share/bison M4=$H/usr/bin/m4 \
    AUTOM4TE=$H/usr/bin/autom4te LD_LIBRARY_PATH=$H/usr/lib/x86_64-linux-gnu PERL5LIB=$H/usr/lib/x86_64-linux-gnu/perl5/5.40:$H/usr/share/perl5 \
    CONVERT=$H/alias/convert RSVG=$H/alias/rsvg ICOTOOL=$H/usr/bin/icotool
if [[ ! -e $W/b/Makefile ]]; then
    mkdir -p "$W/src"; cd "$W"
    base=https://deb.debian.org/debian/pool/main/w/wine
    for f in "wine_${ver%-*}.orig.tar.xz" "wine_$ver.debian.tar.xz"; do [[ -e $f ]] || curl -sSfO "$base/$f"; done
    tar -xf "wine_${ver%-*}.orig.tar.xz" -C src --strip-components=1
    tar -xf "wine_$ver.debian.tar.xz" -C src
    cd src
    while read -r p; do case $p in \#*|"") ;; *) patch -p1 -s -f < "debian/patches/$p" ;; esac; done < debian/patches/series
    patch -p1 -s < "$here/man-native-dialogs.patch"
    ./tools/make_requests; ./tools/make_specfiles
    (cd dlls/opengl32 && ./make_opengl)
    (cd dlls/winevulkan && ./make_vulkan -x "$H/usr/share/vulkan/registry/vk.xml" -X "$H/usr/share/vulkan/registry/video.xml")
    cp "$H/usr/share/misc/config.guess" "$H/usr/share/misc/config.sub" tools/
    sed -n '/^override_dh_auto_configure/,/^$/p' debian/rules | grep -E '^\s+ln -sf .*svg' | sed 's/^\s*//' | sh
    autoconf; autoheader
    for dir in dlls/comdlg32 dlls/shell32; do
        for out in $(grep -hoE '@makedep: [^ ]+\.(ico|bmp)' "$dir"/*.rc | awk '{print $2}' | sort -u); do
            [[ -e $dir/${out%.*}.svg ]] && perl tools/buildimage "$dir/${out%.*}.svg" "$dir/$out"
        done
    done
    mkdir -p ../b/nls nls; cd ../b
    # locale.nls: wine-common's copy (same version), needed by the resource compiler.
    [[ -e nls/locale.nls ]] || dpkg-deb --fsys-tarfile "${WINE_COMMON_DEB:?path to wine-common .deb}" | tar -xO ./usr/share/wine/nls/locale.nls > nls/locale.nls
    cp nls/locale.nls ../src/nls/locale.nls
    CF="-O2 -fno-stack-protector -U_FORTIFY_SOURCE"
    no=(x freetype wayland gnutls alsa dbus fontconfig gstreamer oss pulse sdl udev usb v4l2 vulkan opengl capi cups sane gphoto krb5 netapi opencl osmesa pcap pcsclite ffmpeg)
    ../src/configure x86_64_CFLAGS="$CF" i386_CFLAGS="$CF" --enable-win64 --enable-archs=x86_64,i386 --disable-tests "${no[@]/#/--without-}" || true
    for _ in $(seq 1 80); do
        m=$(./config.status Makefile 2>&1 | grep -oE 'makedep: error: open [^ ]+' | awk '{print $4}') || true
        [[ -n $m ]] || break
        find ../src/dlls ../src/programs ../src/nls ../src/fonts ../src/libs ../src/server -name Makefile.in -print0 | xargs -0 grep -lF "$m" |
            while read -r mk; do d=$(dirname "$mk"); [[ -e $d/$m ]] || { mkdir -p "$(dirname "$d/$m")"; : > "$d/$m"; }; done
    done
fi
cd "$W/b"
make -j"$(nproc)" dlls/comdlg32/{x86_64,i386}-windows/comdlg32.dll dlls/shell32/{x86_64,i386}-windows/shell32.dll
stage=$(mktemp -d); trap 'rm -rf "$stage"' EXIT
L=usr/lib/x86_64-linux-gnu/wine
for a in x86_64-windows i386-windows; do
    install -D -m 0644 "dlls/comdlg32/$a/comdlg32.dll" "$stage/$L/$a/comdlg32.dll"
    install -D -m 0644 "dlls/shell32/$a/shell32.dll" "$stage/$L/$a/shell32.dll"
done
mkdir -p "$stage/DEBIAN" "$stage/usr/share/doc/man-wine-dialogs"
cp "$here/man-native-dialogs.patch" "$here/README.md" "$stage/usr/share/doc/man-wine-dialogs/"
cat > "$stage/DEBIAN/control" <<CTL
Package: man-wine-dialogs
Version: $deb_ver
Architecture: amd64
Maintainer: MAN Developers <dev@man.local>
Section: otherosfs
Priority: optional
Depends: libwine (>= 10.0~), libwine (<< 10.1~), zenity
Description: GNOME's file picker for Windows programs in Wine
 Wine's comdlg32.dll and shell32.dll with MAN's patch (GNU LGPL): file and folder
 dialogs of Windows programs show the desktop's file picker.
CTL
files="$L/x86_64-windows/comdlg32.dll $L/x86_64-windows/shell32.dll $L/i386-windows/comdlg32.dll $L/i386-windows/shell32.dll"
cat > "$stage/DEBIAN/preinst" <<SCR
#!/bin/sh
set -e
for f in $files; do dpkg-divert --package man-wine-dialogs --rename --quiet --divert "/\$f.debian" --add "/\$f"; done
SCR
cat > "$stage/DEBIAN/postrm" <<SCR
#!/bin/sh
set -e
if [ "\$1" = remove ]; then
    for f in $files; do dpkg-divert --package man-wine-dialogs --rename --quiet --remove "/\$f"; done
fi
SCR
chmod 0755 "$stage/DEBIAN/preinst" "$stage/DEBIAN/postrm"
mkdir -p "$here/debs"
dpkg-deb --root-owner-group -Zxz -b "$stage" "$here/debs/man-wine-dialogs_${deb_ver}_amd64.deb"
