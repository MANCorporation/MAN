#!/bin/sh
# Step 2: the MAN installed in step 1 (boot menu has "MAN Recovery").
cd "$(dirname "$0")"
if [ "$(stat -c %s lfs/.vmtest/t7-target.qcow2)" -lt 100000000 ]; then
    echo "The test disk is still empty: run ./test-live.sh and install MAN first." >&2
    exit 1
fi
exec qemu-system-x86_64 -machine q35 -enable-kvm -m 4096 -smp 4 \
  -drive if=pflash,format=raw,unit=0,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,unit=1,file=lfs/.vmtest/t7-vars.fd \
  -drive file=lfs/.vmtest/t7-target.qcow2,format=qcow2,if=virtio \
  -vga virtio -device qemu-xhci -device usb-tablet -nic user,model=virtio-net-pci -display gtk
