# Source me to cross-compile MAN's Rust/GTK apps for ARM64 on the PC:
#   . dev-tools/arm64/cross-env.sh
#   DEB_ARCH=arm64 man-installer/packaging/build-debs.sh
# Needs tools/llvm (duhach/cross/fetch-llvm.sh), the Rust target
# aarch64-unknown-linux-gnu and lfs/arm64/sysroot (GTK 4/libadwaita/libc
# development files for arm64, made by lfs/arm64/share/mksysroot.sh in the
# ARM64 builder and unpacked from sysroot-arm64.tar).
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
root=$(cd -- "$here/../.." && pwd)
. "$root/duhach/cross/env.sh"
sysroot="$root/lfs/arm64/sysroot"
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="$here/aarch64-linux-gnu-clang"
export CC_aarch64_unknown_linux_gnu="$here/aarch64-linux-gnu-clang"
export PKG_CONFIG_ALLOW_CROSS=1
export PKG_CONFIG_SYSROOT_DIR="$sysroot"
export PKG_CONFIG_LIBDIR="$sysroot/usr/lib/aarch64-linux-gnu/pkgconfig:$sysroot/usr/share/pkgconfig"
unset PKG_CONFIG_PATH RUSTFLAGS
