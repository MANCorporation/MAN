#!/usr/bin/env bash
# Cross-compile DUHACH!.exe on Linux and assemble the Windows folder:
#   ./build-windows.sh      -> ../release/DUHACH!-<version>-windows-x64.zip
# Uses tools/mingw (Debian mingw-w64 gcc, unpacked) and tools/gtk-win (GTK 4
# and libadwaita for Windows from MSYS2 ucrt64, fetched by fetch.py).
set -euo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
tools="$here/../tools"
gtk="$tools/gtk-win/root/ucrt64"
export PATH="$tools/mingw/root/usr/bin:$PATH"
export PKG_CONFIG_ALLOW_CROSS=1
export PKG_CONFIG_PATH="$gtk/lib/pkgconfig:$gtk/share/pkgconfig"
export PKG_CONFIG_SYSROOT_DIR="$tools/gtk-win/root"
export PKG_CONFIG_LIBDIR="$gtk/lib/pkgconfig"
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
unset RUSTFLAGS
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$here/Cargo.toml" | head -n1)
cargo build --release --target x86_64-pc-windows-gnu --manifest-path "$here/Cargo.toml"
exe="$here/target/x86_64-pc-windows-gnu/release/duhach.exe"
"$here/package-windows.sh" "$exe" "$version"
