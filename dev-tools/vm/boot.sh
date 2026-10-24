#!/bin/bash
# usage: boot.sh overlay kernel logname
S=$(dirname "$0"); ov=$1; k=$2; log=$3
exec qemu-system-x86_64 -m 4096 -smp 4 -enable-kvm -kernel "$k" \
 -append "root=/dev/vda rootwait console=ttyS0,115200 rw loglevel=4 $EXTRA" \
 -drive file="$ov",format=${FMT:-qcow2},if=virtio -no-reboot \
 -vga virtio -device virtio-keyboard-pci -device virtio-mouse-pci \
 -display none -serial unix:"$S/$log.ser",server,nowait \
 -serial unix:"$S/$log.dbg",server,nowait $EXTRAQ -monitor unix:"$S/$log.mon",server,nowait
