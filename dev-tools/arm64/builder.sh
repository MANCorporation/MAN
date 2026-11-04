#!/bin/bash
# builder.sh: start the ARM64 build machine (emulated, QEMU TCG) in the
# background. Debian 13 arm64 (cloud "nocloud" image, root without password
# on the serial console) with:
#   vda  builder system (overlay, 24 GB, keeps apt caches)
#   vdb  TARGET (default lfs/arm64/man-gnome-arm64.img, raw)
#   9p   lfs/arm64/share  -> mount -t 9p -o trans=virtio share /mnt/share
# Sockets: $RUN/A.serial (shell), A.mon, A.qmp. Stop: echo quit | socat - $RUN/A.mon
set -euo pipefail
p=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
d="$p/lfs/arm64"
RUN=${RUN:-/tmp/claude-1000/arm64}; mkdir -p "$RUN" "$d/share"
q="$p/tools/qemu-arm/root"
target=${TARGET:-$d/man-gnome-arm64.img}
[[ -f $d/builder.qcow2 ]] || { qemu-img create -q -f qcow2 -b debian-13-nocloud-arm64.qcow2 -F qcow2 "$d/builder.qcow2" 24G; }
[[ -f $d/builder-vars.fd ]] || cp "$q/usr/share/AAVMF/AAVMF_VARS.fd" "$d/builder-vars.fd"
[[ -f $target ]] || truncate -s 12G "$target"
exec setsid "$q/usr/bin/qemu-system-aarch64" -machine virt,gic-version=3 -cpu max,pauth-impdef=on -smp ${SMP:-16} -m ${MEM:-16384} \
    -accel tcg,thread=multi,tb-size=1024 \
    -drive if=pflash,format=raw,readonly=on,file="$q/usr/share/AAVMF/AAVMF_CODE.no-secboot.fd" \
    -drive if=pflash,format=raw,file="$d/builder-vars.fd" \
    -drive if=virtio,file="$d/builder.qcow2",format=qcow2,cache=unsafe \
    -drive if=virtio,file="$target",format=raw,cache=unsafe \
    -virtfs local,path="$d/share",mount_tag=share,security_model=none,id=share \
    -nic user,model=virtio-net-pci -device virtio-rng-pci \
    -display none -serial unix:"$RUN/A.serial",server,nowait \
    -monitor unix:"$RUN/A.mon",server,nowait -qmp unix:"$RUN/A.qmp",server,nowait
