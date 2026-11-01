#!/usr/bin/env bash
# Cross-compile DUHACH!.exe on Linux and assemble the Windows folder:
#   ./build-windows.sh          -> ../release/DUHACH!-<version>-windows-x64.zip
#   ./build-windows.sh arm64    -> ../release/DUHACH!-<version>-windows-arm64.zip
# x64 uses tools/mingw (Debian mingw-w64 gcc, unpacked) and MSYS2 ucrt64;
# ARM64 uses tools/llvm (Debian clang/lld, unpacked) and MSYS2 clangarm64.
# GTK 4 and libadwaita come from tools/gtk-win (cross/fetch-gtk.py [clangarm64]);
# the ARM64 compiler from tools/llvm (cross/fetch-llvm.sh).
set -euo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
tools="$here/../tools"
arch=${1:-x64}
case $arch in
    x64)
        env=ucrt64 target=x86_64-pc-windows-gnu
        export PATH="$tools/mingw/root/usr/bin:$PATH"
        export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
        ;;
    arm64)
        env=clangarm64 target=aarch64-pc-windows-gnullvm
        . "$here/cross/env.sh"
        export CARGO_TARGET_AARCH64_PC_WINDOWS_GNULLVM_LINKER=aarch64-w64-mingw32-clang
        export WINDRES="llvm-windres --target=aarch64-w64-mingw32 -I$tools/gtk-win/root/clangarm64/include"
        ;;
    *) echo "usage: $0 [x64|arm64]" >&2; exit 2 ;;
esac
gtk="$tools/gtk-win/root/$env"
export PKG_CONFIG_ALLOW_CROSS=1
export PKG_CONFIG_PATH="$gtk/lib/pkgconfig:$gtk/share/pkgconfig"
export PKG_CONFIG_SYSROOT_DIR="$tools/gtk-win/root"
export PKG_CONFIG_LIBDIR="$gtk/lib/pkgconfig"
unset RUSTFLAGS
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$here/Cargo.toml" | head -n1)
cargo build --release --target "$target" --manifest-path "$here/Cargo.toml"
exe="$here/target/$target/release/duhach.exe"
"$here/package-windows.sh" "$exe" "$version" "$arch"
