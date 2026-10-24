#!/bin/bash
# winvm.sh NAME DISK VARS [CD1] [CD2]  -- Windows test VM (UEFI, SATA, COM1 agent socket NAME.com)
# Sockets and screendumps go to $RUN (default /tmp/claude-1000/winvm).
RUN=${RUN:-/tmp/claude-1000/winvm}; mkdir -p "$RUN"
n=$1; disk=$2; vars=$3; cd1=${4:-}; cd2=${5:-}
args=(-machine q35 -m 6144 -smp 4 -enable-kvm -cpu host
  -drive if=pflash,format=raw,unit=0,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd
  -drive if=pflash,format=raw,unit=1,file="$vars"
  -drive if=none,id=d0,file="$disk",format=qcow2 -device ide-hd,drive=d0,bus=ide.0${BOOTIDX-,bootindex=2}
  -vga ${VGA:-std} -device qemu-xhci -device usb-tablet -display none -nic none
  -serial unix:"$RUN/$n.com",server,nowait -monitor unix:"$RUN/$n.mon",server,nowait
  -qmp unix:"$RUN/$n.qmp",server,nowait -rtc base="${RTC:-2025-03-01T12:00:00}")
[ -n "$cd1" ] && args+=(-drive if=none,id=cd1,media=cdrom,readonly=on,file="$cd1" -device ide-cd,drive=cd1,bus=ide.1${BOOTIDX-,bootindex=1})
[ -n "$cd2" ] && args+=(-drive if=none,id=cd2,media=cdrom,readonly=on,file="$cd2" -device ide-cd,drive=cd2,bus=ide.2)
exec qemu-system-x86_64 "${args[@]}"
