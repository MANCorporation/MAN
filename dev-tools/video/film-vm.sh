#!/bin/bash
# film-vm.sh NAME DISK.qcow2 [ISO]  -- the VM the MAN video is filmed in:
# 1920x1080 (GRUB, start-up screen and desktop), internet, UEFI. With ISO, it
# starts from it as a USB stick. FILMQ=-S starts it paused (film from power-on). Control: NAME.qmp / NAME.mon next to this script.
S=$(cd "$(dirname "$0")" && pwd); n=$1; disk=$2; iso=$3
[ -f "$S/$n-vars.fd" ] || cp /usr/share/OVMF/OVMF_VARS_4M.fd "$S/$n-vars.fd"
stick=()
[ -n "$iso" ] && stick=(-drive if=none,id=st,file=$iso,format=raw,readonly=on -device usb-storage,drive=st,bootindex=1,removable=on)
exec qemu-system-x86_64 -machine q35 -m 8192 -smp 8 -enable-kvm -cpu host \
  -drive if=pflash,format=raw,unit=0,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd -drive if=pflash,format=raw,unit=1,file=$S/$n-vars.fd \
  -device virtio-vga,xres=1920,yres=1080 -device qemu-xhci -device usb-tablet -device virtio-keyboard-pci -display none \
  -nic user,model=virtio-net-pci -device virtio-rng-pci -audiodev none,id=a -device intel-hda -device hda-output,audiodev=a \
  "${stick[@]}" -drive if=none,id=tg,file=$disk,format=qcow2 -device virtio-blk-pci,drive=tg,bootindex=2 \
  $FILMQ -serial file:$S/$n.log -monitor unix:$S/$n.mon,server,nowait -qmp unix:$S/$n.qmp,server,nowait
