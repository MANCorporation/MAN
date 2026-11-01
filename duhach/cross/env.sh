# Source me before building DUHACH! for Windows on ARM: puts Debian's
# clang/lld/llvm tools (unpacked in tools/llvm by fetch-llvm.sh) and the
# aarch64-w64-mingw32-clang wrapper on PATH.
cross=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
llvm="$cross/../../tools/llvm/root"
export LD_LIBRARY_PATH="$llvm/usr/lib/x86_64-linux-gnu:$llvm/usr/lib/llvm-19/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export PATH="$cross:$llvm/usr/lib/llvm-19/bin:$PATH"
