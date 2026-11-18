#!/bin/sh
# The new boot menu on the installed test MAN (a copy: t7-bootmenu.qcow2 is an
# overlay on t7-target.qcow2, which stays as it was; it also has the start-up
# screen, with Plymouth installed by hand). Click into the window,
# then hold Alt from the start until the icons appear.
cd "$(dirname "$0")"
exec qemu-system-x86_64 -machine q35 -enable-kvm -m 4096 -smp 4 \
  -drive if=pflash,format=raw,unit=0,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,unit=1,file=lfs/.vmtest/t7-bootmenu-vars.fd \
  -drive file=lfs/.vmtest/t7-bootmenu.qcow2,format=qcow2,if=none,id=d0 -device virtio-blk-pci,drive=d0,bootindex=0 \
  -vga virtio -device qemu-xhci -device usb-tablet -nic user,model=virtio-net-pci -display gtk
