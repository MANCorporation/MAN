#!/bin/bash
# MAN's GRUB: Debian's grub2 source + man-bootmenu.patch, built rootless.
#   dev-tools/grub-src/build.sh efi|arm64|iso-efi|iso-pc
# Each from the GRUB version it has to match:
#   efi      installed PCs: Debian 2.14-3 (the PC image's modules and grub-install)
#   arm64    ARM and Apple images: Debian 2.14-4
#   iso-efi, iso-pc   the PC ISO (UEFI, BIOS): Debian 2.12-9+deb13u2, as the
#            host's grub-mkrescue that make-iso.sh runs
# Output: build<ver>/stage/usr/lib/grub/<platform>; make-deb.sh packages
# efi and arm64, make-iso.sh takes iso-* from build/stage.
# Build tools that are not on the host (bison, flex, m4, gawk, autotools) come
# from tools/hostbin (Debian packages unpacked there).
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
p=$(cd "$here/../.." && pwd)
what=${1:?efi, arm64, iso-efi or iso-pc}
case $what in
    efi) dir=$here/build2143; dsc=grub2_2.14-3.dsc; ver=2.14-3+man1; plat=efi ;;
    arm64) dir=$here/build214; dsc=grub2_2.14-4.dsc; ver=2.14-4+man1; plat=arm64 ;;
    iso-efi) dir=$here/build; dsc=grub2_2.12-9+deb13u2.dsc; ver=2.12-9+deb13u2+man1; plat=efi ;;
    iso-pc) dir=$here/build; dsc=grub2_2.12-9+deb13u2.dsc; ver=2.12-9+deb13u2+man1; plat=pc ;;
    *) echo "efi, arm64, iso-efi or iso-pc" >&2; exit 2 ;;
esac
H=$p/tools/hostbin/root
mkdir -p "$dir/bin"

# Autotools from tools/hostbin: aclocal/automake point at /usr/share, so they
# get copies with the paths changed; autoconf's are already relocated there.
for t in aclocal-1.17 automake-1.17; do
    [ -x "$dir/bin/$t" ] || { sed "s#/usr/share#$H/usr/share#g" "$H/usr/bin/$t" > "$dir/bin/$t"; chmod +x "$dir/bin/$t"; }
done
for t in autoconf autoheader autom4te autoreconf; do [ -x "$dir/bin/$t" ] || cp "$H/usr/bin/$t" "$dir/bin/$t"; done
for l in aclocal:aclocal-1.17 automake:automake-1.17 aclocal-1.15:aclocal-1.17 automake-1.15:automake-1.17; do
    [ -e "$dir/bin/${l%%:*}" ] || ln -s "${l#*:}" "$dir/bin/${l%%:*}"
done
export PATH=$dir/bin:$H/usr/bin:$PATH
export LD_LIBRARY_PATH=$H/usr/lib/x86_64-linux-gnu
export BISON_PKGDATADIR=$H/usr/share/bison M4=$H/usr/bin/m4
export AUTOM4TE=$dir/bin/autom4te AUTOCONF=$dir/bin/autoconf AUTOHEADER=$dir/bin/autoheader
export ACLOCAL=$dir/bin/aclocal AUTOMAKE=$dir/bin/automake AUTOMAKE_LIBDIR=$H/usr/share/automake-1.17
export ACLOCAL_PATH=$H/usr/share/aclocal:/usr/share/aclocal
export AUTOPOINT=true LIBTOOLIZE=true

if [ ! -d "$dir/grub2" ]; then
    dpkg-source -x "$here/$dsc" "$dir/grub2"
    (cd "$dir/grub2" && patch -p1 < "$here/man-bootmenu.patch")
fi
if [ ! -f "$dir/grub2/.man-autogen" ]; then
    (cd "$dir/grub2" && ./autogen.sh) > "$dir/autogen.log" 2>&1
    touch "$dir/grub2/.man-autogen"
fi

conf=(PACKAGE_VERSION="$ver" PACKAGE_STRING="GRUB $ver" --prefix=/usr --libdir=/usr/lib --libexecdir=/usr/lib
      --disable-grub-mkfont --disable-grub-themes --disable-device-mapper --disable-liblzma --disable-libzfs
      --disable-grub-mount --disable-werror)
case $plat in
    pc) conf+=(--with-platform=pc --target=i386) ;;
    efi) conf+=(--with-platform=efi --target=x86_64) ;;
    arm64)
        G=$p/tools/gcc-arm64/root
        export PATH=$G/usr/bin:$PATH LD_LIBRARY_PATH=$G/usr/lib/x86_64-linux-gnu:$LD_LIBRARY_PATH
        conf+=(--with-platform=efi --target=aarch64-linux-gnu TARGET_CC=aarch64-linux-gnu-gcc
               TARGET_OBJCOPY=aarch64-linux-gnu-objcopy TARGET_STRIP=aarch64-linux-gnu-strip
               TARGET_NM=aarch64-linux-gnu-nm TARGET_RANLIB=aarch64-linux-gnu-ranlib) ;;
esac
mkdir -p "$dir/obj-$plat"
cd "$dir/obj-$plat"
[ -f Makefile ] || ../grub2/configure "${conf[@]}" > "../configure-$plat.log" 2>&1
make -j"$(nproc)" -C grub-core AWK=gawk > "../make-$plat.log" 2>&1
make -C grub-core install DESTDIR="$dir/stage" AWK=gawk > "../install-$plat.log" 2>&1
find "$dir/stage" \( -name "*.module" -o -name kernel.exec -o -name "gdb_*" \) -delete
echo "$plat: $(ls "$dir"/stage/usr/lib/grub/*/kernel.img | wc -l) platform(s) in $dir/stage"
