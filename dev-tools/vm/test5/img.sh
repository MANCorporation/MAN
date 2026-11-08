#!/bin/bash
# Set W to a work dir (holds vmlinuz from the image: debugfs -R "dump /boot/vmlinuz-7.3.0-rc7 $W/vmlinuz" lfs/.vmtest/test4.img), P to the repo.
# Edit the real x86 image: no graphics, root shell on serial 2, packages via 9p.
exec qemu-system-x86_64 -m 4096 -smp 4 -enable-kvm -cpu host -kernel $W/vmlinuz \
 -append "root=/dev/vda rootwait rw quiet systemd.debug_shell=ttyS1 systemd.unit=multi-user.target" \
 -drive file=$P/lfs/.vmtest/test4.img,format=raw,if=virtio -display none -serial null -serial unix:$W/ish,server,nowait \
 -monitor unix:$W/im,server,nowait -nic none \
 -virtfs local,path=$W/share,mount_tag=w,security_model=none,readonly=on
