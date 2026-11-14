#!/bin/bash
# Boot the Mac install disk (lfs/apple/disk.img) in an emulated ARM64 PC (QEMU TCG, cpu max:
# 16K pages work) through its own GRUB, on a throwaway overlay. No Mac hardware here: this
# checks GRUB, the Asahi kernel's generic parts, initramfs, systemd and the desktop.
# Sockets in /tmp/claude-1000/apple: B.serial, B.mon, B.qmp (screendump for screenshots).
set -euo pipefail
p=$(cd -- "$(dirname -- "$(readlink -f "$0")")/../.." && pwd)
q=$p/tools/qemu-arm/root d=$p/lfs/apple R=${RUN:-/tmp/claude-1000/apple}; mkdir -p $R
cp $q/usr/share/AAVMF/AAVMF_VARS.fd $d/test-vars.fd
rm -f $d/test.qcow2; qemu-img create -q -f qcow2 -b disk.img -F raw $d/test.qcow2
exec setsid $q/usr/bin/qemu-system-aarch64 -machine virt,gic-version=3 -cpu max,pauth-impdef=on -smp 8 -m 8192 \
  -accel tcg,thread=multi,tb-size=1024 \
  -drive if=pflash,format=raw,readonly=on,file=$q/usr/share/AAVMF/AAVMF_CODE.no-secboot.fd \
  -drive if=pflash,format=raw,file=$d/test-vars.fd -drive if=virtio,file=$d/test.qcow2,format=qcow2 \
  -device virtio-gpu-pci,xres=1280,yres=800 -device qemu-xhci -device usb-kbd -device usb-tablet \
  -nic user,model=virtio-net-pci -device virtio-rng-pci -display none \
  -serial unix:$R/B.serial,server,nowait -monitor unix:$R/B.mon,server,nowait -qmp unix:$R/B.qmp,server,nowait
