#!/bin/bash
# MAN's copy of the Asahi installer (MIT) for release/apple and the USB stick:
#   installer/patch-urlcache.py  HTTP status checks, fresh GitHub signed links
#   installer/patch-auto.py      shuts down / restarts by itself, no extra key presses
# Output: release/apple/installer-<ver>-man2.tar.gz and release/apple/latest
set -euo pipefail
p=$(cd -- "$(dirname -- "$(readlink -f "$0")")/../.." && pwd)
A=$p/lfs/apple; OUT=$p/release/apple
IV=v0.9.2; MV=$IV-man2
mkdir -p $A/installer $OUT
[ -f $A/installer/installer-$IV.tar.gz ] || curl -sfL -o $A/installer/installer-$IV.tar.gz https://cdn.asahilinux.org/installer/installer-$IV.tar.gz
rm -rf $A/installer/tree; mkdir -p $A/installer/tree
tar -C $A/installer/tree -xzf $A/installer/installer-$IV.tar.gz
python3 -I $p/dev-tools/apple/installer/patch-urlcache.py $A/installer/tree/urlcache.py
python3 -I $p/dev-tools/apple/installer/patch-auto.py $A/installer/tree
for f in $OUT/installer-*.tar.gz; do [ -e "$f" ] && rm -f "$f"; done
tar -C $A/installer/tree --owner=0 --group=0 -czf $OUT/installer-$MV.tar.gz .
echo $MV > $OUT/latest
ls -la $OUT/installer-$MV.tar.gz
