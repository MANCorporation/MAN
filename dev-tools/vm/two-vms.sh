#!/bin/bash
# vm2.sh DISK DIR IPTAIL: VM on a private network shared with the other one
cd ~/Documents/MANRUST
rm -f $2/s $2/m $2/q
exec qemu-system-x86_64 -enable-kvm -cpu host -m 5G -smp 4 -kernel lfs/boot/vmlinuz-7.3-rc7 -append "root=/dev/vda rootwait rw quiet systemd.debug_shell=ttyS1" -drive file=$1,format=qcow2,if=virtio -device virtio-vga,xres=1920,yres=1056 -device qemu-xhci -device usb-tablet -device virtio-keyboard-pci -netdev socket,id=l,mcast=230.0.77.1:17765 -device virtio-net-pci,netdev=l,mac=52:54:00:77:00:$3 -virtfs local,path=$SCR/w,mount_tag=w,security_model=none -serial null -serial unix:$2/s,server,nowait -monitor unix:$2/m,server,nowait -qmp unix:$2/q,server,nowait -display none
