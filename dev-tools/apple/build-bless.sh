#!/bin/bash
# asahi-bless (The Asahi Linux Contributors, MIT) for ARM64, unchanged from crates.io:
# MAN's Startup Disk uses it to set the Mac's start-up disk (NVRAM).
# Output: lfs/arm64/share/apple/asahi-bless (installed by guest/apple-system.sh).
set -euo pipefail
p=$(cd -- "$(dirname -- "$(readlink -f "$0")")/../.." && pwd)
V=0.4.4; W=$p/lfs/apple/bless-src
mkdir -p $W
[ -d $W/asahi-bless-$V ] || { curl -sfL -A build-script -o $W/asahi-bless-$V.crate \
  https://crates.io/api/v1/crates/asahi-bless/$V/download; tar -C $W -xzf $W/asahi-bless-$V.crate; }
. $p/dev-tools/arm64/cross-env.sh
(cd $W/asahi-bless-$V && cargo build --release --locked --target aarch64-unknown-linux-gnu)
install -m 755 $W/asahi-bless-$V/target/aarch64-unknown-linux-gnu/release/asahi-bless $p/lfs/arm64/share/apple/asahi-bless
llvm-strip $p/lfs/arm64/share/apple/asahi-bless 2>/dev/null || $p/tools/llvm/usr/bin/llvm-strip $p/lfs/arm64/share/apple/asahi-bless 2>/dev/null || true
ls -la $p/lfs/arm64/share/apple/asahi-bless
