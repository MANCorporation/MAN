#!/bin/bash
# Set W to a work dir (holds vmlinuz from the image: debugfs -R "dump /boot/vmlinuz-7.3.0-rc7 $W/vmlinuz" lfs/.vmtest/test4.img), P to the repo.
# Throwaway test VM: test4.img overlay, graphical, root shell on serial 2, QMP + monitor sockets.
exec qemu-system-x86_64 -m 6144 -smp 4 -enable-kvm -cpu host -kernel $W/vmlinuz \
 -append "root=/dev/vda rootwait rw quiet systemd.debug_shell=ttyS1" \
 -drive file=$W/ov.qcow2,format=qcow2,if=virtio \
 -device virtio-vga,xres=1920,yres=1056 -device virtio-keyboard-pci -device virtio-tablet-pci \
 -display none -serial null -serial unix:$W/sh,server,nowait \
 -qmp unix:$W/q,server,nowait -monitor unix:$W/m,server,nowait \
 -nic user,model=virtio-net-pci \
 -virtfs local,path=$W/share,mount_tag=w,security_model=none,readonly=on
