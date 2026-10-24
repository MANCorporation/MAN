#!/bin/bash
# make-devroot.sh: GTK 4 / libadwaita development files without root, in
# <this dir>/devroot (use with buildenv.sh). Recreate after a reboot (/tmp).
set -e
S=$(cd "$(dirname "$0")" && pwd); D=$S/devroot
mkdir -p $S/devdebs $D && cd $S/devdebs
apt-get download libgtk-4-dev libadwaita-1-dev libglib2.0-dev libgio-2.0-dev libglib2.0-dev-bin libgirepository-2.0-dev gir1.2-glib-2.0-dev libpango1.0-dev libcairo2-dev libgdk-pixbuf-2.0-dev libgraphene-1.0-dev libharfbuzz-dev libfreetype-dev libfontconfig-dev libfribidi-dev libpixman-1-dev libpng-dev libxft-dev libx11-dev libxrender-dev libxext-dev libthai-dev libdatrie-dev libffi-dev libmount-dev libblkid-dev libselinux1-dev libsepol-dev libpcre2-dev zlib1g-dev libsysprof-capture-4-dev libappstream-dev libbrotli-dev libexpat1-dev libxcb1-dev libxau-dev libxdmcp-dev x11proto-dev libxcb-render0-dev libxcb-shm0-dev liblzo2-dev libbz2-dev libjpeg62-turbo-dev libtiff-dev libwebp-dev libvulkan-dev libepoxy-dev libxkbcommon-dev libwayland-dev libxi-dev libxfixes-dev libxrandr-dev libxcursor-dev libxdamage-dev libxinerama-dev libgraphite2-dev liblerc-dev libdeflate-dev liblzma-dev libsharpyuv-dev libzstd-dev >/dev/null
for f in *.deb; do dpkg-deb -x $f $D; done
# The .so links point inside the sysroot; the libraries are the host's.
cd $D/usr/lib/x86_64-linux-gnu
for l in *.so; do [ -L "$l" ] && [ ! -e "$l" ] && { t=$(readlink "$l"); [ -e /usr/lib/x86_64-linux-gnu/$t ] && ln -sf /usr/lib/x86_64-linux-gnu/$t "$l"; }; done
cat > $S/buildenv.sh <<ENV
export PKG_CONFIG_PATH=$D/usr/lib/x86_64-linux-gnu/pkgconfig:$D/usr/share/pkgconfig
export PKG_CONFIG_SYSROOT_DIR=$D
export RUSTFLAGS="-L $D/usr/lib/x86_64-linux-gnu"
ENV
echo "devroot ready: . $S/buildenv.sh"
