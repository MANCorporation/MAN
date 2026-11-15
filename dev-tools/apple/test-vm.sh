#!/bin/bash
# Boot the Mac install disk (lfs/apple/disk.img) in an emulated ARM64 PC (QEMU TCG, cpu max:
# 16K pages work) through its own GRUB, on a throwaway overlay. No Mac hardware here: this
# checks GRUB, the Asahi kernel's generic parts, initramfs, systemd and the desktop.
# Sockets in /tmp/claude-1000/apple: B.serial, B.mon, B.qmp (screendump for screenshots).
#   test-vm.sh        the installed disk (lfs/apple/disk.img)
#   test-vm.sh iso    the newest release/MAN-*-apple.iso as a USB stick, empty disk
#   FAKEMAC=1 test-vm.sh iso   also lfs/apple/fakemac.img as the "internal" NVMe disk and
#                     no ACPI (device tree), to test the installer's Mac mode (fake the
#                     Mac in the live session: see README "Testing the Mac mode")
set -euo pipefail
p=$(cd -- "$(dirname -- "$(readlink -f "$0")")/../.." && pwd)
q=$p/tools/qemu-arm/root d=$p/lfs/apple R=${RUN:-/tmp/claude-1000/apple}; mkdir -p $R
cp $q/usr/share/AAVMF/AAVMF_VARS.fd $d/test-vars.fd
rm -f $d/test.qcow2
drives=(-drive if=virtio,file=$d/test.qcow2,format=qcow2)
if [ "${1:-}" = iso ]; then
  iso=$(ls -t $p/release/MAN-*-apple.iso | head -n1)
  qemu-img create -q -f qcow2 $d/test.qcow2 32G
  drives+=(-drive if=none,id=stick,format=raw,readonly=on,file=$iso -device usb-storage,drive=stick,removable=on)
else
  qemu-img create -q -f qcow2 -b disk.img -F raw $d/test.qcow2
fi
machine=virt,gic-version=3
if [ -n "${FAKEMAC:-}" ]; then
  machine+=,acpi=off
  drives+=(-drive if=none,id=mac,format=raw,file=$d/fakemac.img -device nvme,drive=mac,serial=FAKEMAC)
fi
exec setsid $q/usr/bin/qemu-system-aarch64 -machine $machine -cpu max,pauth-impdef=on -smp 8 -m 8192 \
  -accel tcg,thread=multi,tb-size=1024 \
  -drive if=pflash,format=raw,readonly=on,file=$q/usr/share/AAVMF/AAVMF_CODE.no-secboot.fd \
  -drive if=pflash,format=raw,file=$d/test-vars.fd \
  -device virtio-gpu-pci,xres=1280,yres=800 -device qemu-xhci -device usb-kbd -device usb-tablet \
  -nic user,model=virtio-net-pci -device virtio-rng-pci -display none \
  -serial unix:$R/B.serial,server,nowait -monitor unix:$R/B.mon,server,nowait -qmp unix:$R/B.qmp,server,nowait \
  "${drives[@]}"
