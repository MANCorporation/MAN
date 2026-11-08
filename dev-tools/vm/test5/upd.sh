#!/bin/bash
# Set W to a work dir (holds vmlinuz from the image: debugfs -R "dump /boot/vmlinuz-7.3.0-rc7 $W/vmlinuz" lfs/.vmtest/test4.img), P to the repo.
# Update payload VM: a throwaway copy of the image boots (vda); the image itself is vdb, read-only.
exec qemu-system-x86_64 -m 6144 -smp 6 -enable-kvm -cpu host -kernel $W/vmlinuz \
 -append "root=/dev/vda rootwait rw quiet systemd.debug_shell=ttyS1 systemd.unit=multi-user.target" \
 -drive file=$W/upd-root.qcow2,format=qcow2,if=virtio \
 -drive file=$P/lfs/.vmtest/test4.img,format=raw,if=virtio,readonly=on \
 -display none -serial null -serial unix:$W/ush,server,nowait -monitor unix:$W/um,server,nowait -nic none \
 -virtfs local,path=$W/out,mount_tag=out,security_model=none
