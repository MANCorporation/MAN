#!/bin/bash
# uefi.sh NAME VARS [kernel] -- disks via DISKS="file:fmt ..." ; UEFI (OVMF) VM with serial sockets
S=$(dirname "$0"); name=$1; vars=$2; kernel=${3:-}
args=(-machine q35 -m 4096 -smp 4 -enable-kvm
  -drive if=pflash,format=raw,unit=0,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd
  -drive if=pflash,format=raw,unit=1,file="$vars"
  -vga virtio -device qemu-xhci -device usb-tablet -device virtio-keyboard-pci
  -display none ${NOREBOOT--no-reboot} -nic none
  -serial unix:"$S/$name.ser",server,nowait -serial unix:"$S/$name.dbg",server,nowait
  -monitor unix:"$S/$name.mon",server,nowait -qmp unix:"$S/$name.qmp",server,nowait)
for d in $DISKS; do args+=(-drive "file=${d%%:*},format=${d##*:},if=virtio"); done
[ -n "$kernel" ] && args+=(-kernel "$kernel" -append "root=/dev/vda rootwait console=ttyS0,115200 rw loglevel=4 $EXTRA")
exec qemu-system-x86_64 "${args[@]}" $EXTRAQ
