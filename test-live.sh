#!/bin/sh
# Step 1: MAN Live (new login screen + MAN Recovery) with an empty 40 GB disk to install onto.
cd "$(dirname "$0")"
exec qemu-system-x86_64 -machine q35 -enable-kvm -m 4096 -smp 4 \
  -drive if=pflash,format=raw,unit=0,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,unit=1,file=lfs/.vmtest/t7-vars.fd \
  -kernel lfs/boot/vmlinuz-7.3-rc7 -append "root=/dev/vda rw quiet" \
  -drive file=lfs/.vmtest/t7-live.qcow2,format=qcow2,if=virtio \
  -drive file=lfs/.vmtest/t7-target.qcow2,format=qcow2,if=virtio \
  -vga virtio -device qemu-xhci -device usb-tablet -nic user,model=virtio-net-pci -display gtk
