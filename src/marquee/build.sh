#!/bin/bash
# build.sh [amd64|arm64]: Marquee's two programs at the commit below + share-on-man.patch,
# into src/marquee/out/ARCH, where man-installer/packaging/build-debs.sh packs them.
# amd64 needs alsa.pc (PKG_CONFIG_PATH); arm64: first . dev-tools/arm64/cross-env.sh
set -euo pipefail
COMMIT=b5fddf1b3332b45ba48c0e38401f98509d73b53f
REPO=${MARQUEE_REPO:-https://github.com/MANCorporation/Marquee.git}
here=$(cd -- "$(dirname -- "$0")" && pwd)
arch=${1:-amd64}
case $arch in
    amd64) target= ; rel=release ;;
    arm64) target="--target aarch64-unknown-linux-gnu"; rel=aarch64-unknown-linux-gnu/release ;;
    *) echo "amd64 or arm64" >&2; exit 2 ;;
esac
work=$(mktemp -d); trap 'rm -rf "$work"' EXIT
git clone -q "$REPO" "$work/src"
git -C "$work/src" checkout -q "$COMMIT"
git -C "$work/src" apply "$here/share-on-man.patch"
export CARGO_TARGET_DIR=${MARQUEE_TARGET:-$here/target}
cargo build --release --locked --manifest-path "$work/src/Cargo.toml" $target
out=$here/out/$arch
rm -rf "$out"; mkdir -p "$out"
install -m 0755 "$CARGO_TARGET_DIR/$rel/marquee" "$CARGO_TARGET_DIR/$rel/marquee-share" "$out/"
cp -r "$work/src/assets/appicon" "$out/appicon"
cp "$work/src/LICENSE" "$work/src/THIRD_PARTY_NOTICES.md" "$out/"
echo "built $out"
