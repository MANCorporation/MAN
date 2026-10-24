#!/bin/bash
# fresh-install.sh : no-account install of live4.img onto a blank /home/kristihack/Documents/MANRUST/lfs/.vmtest/o-target.qcow2
S=$(cd "$(dirname "$0")" && pwd); R=$S/repo2; cd $S
rm -f o-live.qcow2 /home/kristihack/Documents/MANRUST/lfs/.vmtest/o-target.qcow2; qemu-img create -f qcow2 -F raw -b $S/live4.img o-live.qcow2 >/dev/null; qemu-img create -f qcow2 /home/kristihack/Documents/MANRUST/lfs/.vmtest/o-target.qcow2 40G >/dev/null; cp /usr/share/OVMF/OVMF_VARS_4M.fd /home/kristihack/Documents/MANRUST/lfs/.vmtest/o-vars.fd
EXTRAQ="-virtfs local,path=$R,mount_tag=repo,security_model=none,readonly=on" DISKS="$S/o-live.qcow2:qcow2 /home/kristihack/Documents/MANRUST/lfs/.vmtest/o-target.qcow2:qcow2" EXTRA="systemd.debug_shell=ttyS1 systemd.unit=multi-user.target" nohup setsid ./uefi.sh FI /home/kristihack/Documents/MANRUST/lfs/.vmtest/o-vars.fd $R/kernel/vmlinuz-7.3.0-rc7 >FI.qemuerr 2>&1 &
for i in $(seq 40); do sleep 3; python3 -I ser.py FI.dbg "echo READY-\$((6*7))" 2 2>/dev/null | grep -q READY-42 && break; done
python3 -I ser.py FI.dbg "mkdir -p /mnt/repo && mount -t 9p -o trans=virtio,ro,msize=262144 repo /mnt/repo; /mnt/repo/run-plan.sh /mnt/repo/plans/o1-noaccount.json | tail -2; umount /mnt/repo; sync; systemctl poweroff" 400 | grep -E 'DONE|ERROR|exit='
for i in $(seq 60); do pgrep -x qemu-system-x86 >/dev/null || break; sleep 1; done
