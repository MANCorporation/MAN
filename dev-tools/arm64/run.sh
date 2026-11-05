#!/bin/bash
# run.sh MODE: start MAN ARM64 in QEMU (emulated, slow) without a window;
# screen via QMP screendump (sockets in $RUN: B.qmp, B.mon, B.serial).
# CPU=cortex-a76 for a CPU like real ARM PCs have: -cpu max has SVE/SME, and Wine
# dies there on an illegal instruction. EXTRA= more kernel options (debug mode).
#   run.sh image   the system image directly (kernel + initrd from lfs/arm64/boot)
#   run.sh iso     the ARM64 ISO (release/MAN-*-arm64.iso) with an empty disk
#   run.sh disk    what was installed on that disk, through its own GRUB (no ISO)
#   run.sh debug   like iso, but its kernel and initrd are started directly with a
#                  root shell on the serial port (lfs/arm64/boot/iso-initrd.img)
set -euo pipefail
p=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
d="$p/lfs/arm64"; q="$p/tools/qemu-arm/root"
RUN=${RUN:-/tmp/claude-1000/arm64}; mkdir -p "$RUN"
mode=${1:-image}
args=(-machine virt,gic-version=3 -cpu ${CPU:-max,pauth-impdef=on} -smp ${SMP:-8} -m ${MEM:-8192}
  -accel tcg,thread=multi,tb-size=1024
  -drive if=pflash,format=raw,readonly=on,file="$q/usr/share/AAVMF/AAVMF_CODE.no-secboot.fd"
  -device virtio-gpu-pci,xres=1280,yres=800 -device qemu-xhci -device usb-kbd -device usb-tablet
  -nic user,model=virtio-net-pci -device virtio-rng-pci -display none
  -serial unix:"$RUN/B.serial",server,nowait -monitor unix:"$RUN/B.mon",server,nowait -qmp unix:"$RUN/B.qmp",server,nowait)
case $mode in
  image)
    [[ -f $d/test-vars.fd ]] || cp "$q/usr/share/AAVMF/AAVMF_VARS.fd" "$d/test-vars.fd"
    qemu-img create -q -f qcow2 -b man-gnome-arm64.img -F raw "$d/test.qcow2"
    args+=(-drive if=pflash,format=raw,file="$d/test-vars.fd" -drive if=virtio,file="$d/test.qcow2",format=qcow2
      -kernel "$d/boot/vmlinuz" -initrd "$d/boot/initrd.img" -append "root=/dev/vda rw console=ttyAMA0 console=tty0 systemd.debug_shell=ttyAMA0") ;;
  disk)
    args+=(-drive if=pflash,format=raw,file="$d/iso-vars.fd" -drive if=virtio,file="$d/iso-disk.qcow2",format=qcow2) ;;
  iso|debug)
    iso=$(ls -t "$p"/release/MAN-*-arm64.iso | head -n1)
    cp "$q/usr/share/AAVMF/AAVMF_VARS.fd" "$d/iso-vars.fd"
    rm -f "$d/iso-disk.qcow2"; qemu-img create -q -f qcow2 "$d/iso-disk.qcow2" 64G
    args+=(-drive if=pflash,format=raw,file="$d/iso-vars.fd" -drive if=virtio,file="$d/iso-disk.qcow2",format=qcow2
      -drive if=none,id=cd,media=cdrom,readonly=on,file="$iso" -device usb-storage,drive=cd,removable=on)
    [[ $mode == debug ]] && args+=(-kernel "$d/boot/vmlinuz" -initrd "$d/boot/iso-initrd.img" -append "console=ttyAMA0 console=tty0 systemd.debug_shell=ttyAMA0 ${EXTRA:-}")
    ;;
esac
exec setsid "$q/usr/bin/qemu-system-aarch64" "${args[@]}"
