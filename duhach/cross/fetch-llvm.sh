#!/usr/bin/env bash
# Get Debian's clang, lld and llvm tools (19) and unpack them into
# tools/llvm/root, without installing anything on the system.
set -euo pipefail
dest=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../tools" && pwd)/llvm
mkdir -p "$dest/debs" "$dest/root"
cd "$dest/debs"
apt-get download clang-19 lld-19 llvm-19 llvm-19-linker-tools libclang-cpp19 libllvm19 liblld-19 libclang-common-19-dev
for d in *.deb; do dpkg-deb -x "$d" "$dest/root"; done
echo "LLVM unpacked in $dest/root"
