#!/bin/bash
# iso-vm.sh NAME ISO TARGET [bios]  -- boot the ISO as a USB stick, serial log to NAME.log
S=$(dirname "$0"); n=$1; iso=$2; tgt=$3
fw=(-drive if=pflash,format=raw,unit=0,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd -drive if=pflash,format=raw,unit=1,file=$S/$n-vars.fd)
[ "$4" = bios ] && fw=()
exec qemu-system-x86_64 -machine q35 -m 4096 -smp 4 -enable-kvm "${fw[@]}" \
  -vga virtio -device qemu-xhci -device usb-tablet -device virtio-keyboard-pci -display none -nic none \
  -drive if=none,id=st,file=$iso,format=raw,readonly=on -device usb-storage,drive=st,bootindex=1,removable=on \
  -drive if=none,id=tg,file=$tgt,format=qcow2 -device virtio-blk-pci,drive=tg,bootindex=2 \
  -serial file:$S/$n.log -monitor unix:$S/$n.mon,server,nowait -qmp unix:$S/$n.qmp,server,nowait
