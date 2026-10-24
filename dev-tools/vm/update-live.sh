#!/bin/bash
# update-live.sh : rebuild debs and install them into live6.img (in a VM, as root)
S=$(cd "$(dirname "$0")" && pwd); R=$S/repo2; cd /home/kristihack/Documents/MANRUST/man-installer; . $S/buildenv.sh
packaging/build-debs.sh 2>&1 | grep -E 'built|error'; cp target/debs/*.deb $R/; (cd $R && apt-ftparchive packages . > Packages 2>/dev/null)
cd $S
EXTRAQ="-nic none -virtfs local,path=$R,mount_tag=repo,security_model=none,readonly=on" \
FMT=raw EXTRA="systemd.debug_shell=ttyS1 systemd.unit=multi-user.target" nohup setsid ./boot.sh $S/live6.img $R/kernel/vmlinuz-7.3.0-rc7 prep >prep.qemuerr 2>&1 &
for i in $(seq 30); do sleep 3; python3 -I ser.py prep.dbg "echo READY-\$((6*7))" 2 2>/dev/null | grep -q READY-42 && break; done
python3 -I ser.py prep.dbg "mkdir -p /mnt/repo && mount -t 9p -o trans=virtio,ro,msize=262144 repo /mnt/repo && bash /mnt/repo/live-prep.sh > /root/prep.log 2>&1; echo PREP_EXIT=\$?; rm -f /root/prep.log /var/log/man-installer.log; umount /mnt/repo; rmdir /mnt/repo; sync; systemctl poweroff" 400 | grep -E 'PREP_EXIT'
for i in $(seq 60); do pgrep -x qemu-system-x86 >/dev/null || break; sleep 1; done; /sbin/e2fsck -fn live6.img 2>&1 | tail -1
