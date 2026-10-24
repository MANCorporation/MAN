#!/usr/bin/env bash
# Build lfs/updates-stick.img: a FAT32 "USB stick" holding every lfs/updates/*.upd.
# run-gnome.sh plugs it into the VM in "installed" mode; GNOME shows it in Files.
set -euo pipefail
project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
updates="$project_dir/lfs/updates"
out="$project_dir/lfs/updates-stick.img"
PATH="$project_dir/tools/mtools/usr/bin:$PATH"
shopt -s nullglob
files=("$updates"/*.upd)
(( ${#files[@]} )) || { echo "No .upd files in $updates" >&2; exit 1; }
bytes=$(du -cb "${files[@]}" | tail -1 | cut -f1)
mib=$(( bytes / 1048576 + 128 ))
rm -f "$out.tmp"
/sbin/mkfs.fat -C -F 32 -n MAN_UPDATES "$out.tmp" $(( mib * 1024 )) >/dev/null
for f in "${files[@]}"; do mcopy -i "$out.tmp" "$f" ::/; done
mv "$out.tmp" "$out"
echo "Built $out with: $(basename -a "${files[@]}" | tr '\n' ' ')"
