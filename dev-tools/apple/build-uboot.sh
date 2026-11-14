#!/bin/bash
# Asahi U-Boot (lfs/apple/u-boot = AsahiLinux/u-boot tag asahi-v2026.07-3) with
# Debian's GCC cross compiler unpacked in tools/gcc-arm64. Output lfs/apple/ub/u-boot-nodtb.bin
set -e
p=$(cd -- "$(dirname -- "$(readlink -f "$0")")/../.." && pwd)
export PATH=$p/tools/gcc-arm64/root/usr/bin:$p/tools/hostbin/root/usr/bin:$PATH
export BISON_PKGDATADIR=$p/tools/hostbin/root/usr/share/bison M4=$p/tools/hostbin/root/usr/bin/m4
export LD_LIBRARY_PATH=$p/tools/gcc-arm64/root/usr/lib/x86_64-linux-gnu
cd $p/lfs/apple/u-boot
U="make O=../ub CROSS_COMPILE=aarch64-linux-gnu-"
rm -rf ../ub
$U apple_m1_defconfig
scripts/config --file ../ub/.config -d TOOLS_MKEFICAPSULE -d TOOLS_LIBCRYPTO -d TOOLS_KWBIMAGE
$U olddefconfig
$U -j8
ls -la ../ub/u-boot-nodtb.bin
