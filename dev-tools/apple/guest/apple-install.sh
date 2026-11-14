#!/bin/bash
# In the ARM64 builder, after apple-system.sh: install the Mac system (/dev/vdb) onto
# the install disk /dev/vdc with MAN's own installer backend, exactly like a normal
# MAN installation ("Erase Disk", MAN Setup on the first start), so the Mac gets the
# usual layout: Btrfs @system/@home/@snapshots, .upd updates and snapshots working, and a
# separate /boot so MAN Setup can encrypt the system.
#   vdc1  EFI (FAT32)        -> becomes the esp/ folder of the Asahi installer package
#   vdc2  MAN_BOOT (ext4)    -> boot.img
#   vdc3  MAN (Btrfs)        -> root.img
# The builder's own UEFI variables get a "MAN" boot entry from grub-install: it is
# removed again at the end.
set -euo pipefail
T=/mnt/target D=/dev/vdc
mkdir -p $T; mountpoint -q $T || mount /dev/vdb $T

echo "== clear $D"
wipefs -a $D >/dev/null

mount -t proc proc $T/proc; mount -t sysfs sys $T/sys; mount --rbind /dev $T/dev; mount -t tmpfs tmpfs $T/run
mkdir -p $T/run/udev; mount --bind /run/udev $T/run/udev
trap 'umount -R $T/run $T/dev $T/sys $T/proc 2>/dev/null || true' EXIT
B=$(chroot $T sh -c 'ls /usr/libexec/man-installer/man-installer-backend /usr/bin/man-installer-backend 2>/dev/null | head -1')
echo "backend: $B"

echo "== probe"
chroot $T $B request <<<'{"op":"probe"}' > /tmp/probe.json
python3 - <<'EOF' > /tmp/plan.json
import json
probe = None
for line in open('/tmp/probe.json'):
    e = json.loads(line)
    if e.get('type') == 'probe':
        probe = e['probe']
    if e.get('type') == 'error':
        raise SystemExit(f"probe failed: {e}")
disk = next(d for d in probe['disks'] if d['path'] == '/dev/vdc')
assert not disk['is_live'], disk
plan = dict(mode='erase', disk=disk['path'], confirm=f"{disk['path']}|{disk['size']}|{disk['serial']}",
            esp=None, format_esp=True, root=None, boot=None, passphrase=None, encrypt=False,
            user=None, root_password=None, hostname='man', timezone='Etc/UTC', keyboard='us', language=None)
print(json.dumps({'op': 'install', 'plan': plan}))
EOF
cat /tmp/plan.json; echo

echo "== install"
# The image is packed with LZMA (GitHub allows 2 GiB per file): copy without Btrfs
# compression, LZMA compresses plain data much better. The fstab keeps compress=zstd:1,
# so files the Mac writes later are compressed as usual.
mkdir -p $T/tmp/man-nocompress
cat > $T/tmp/man-nocompress/mount <<'W'
#!/bin/bash
a=(); for x in "$@"; do a+=("${x//compress=zstd:1,/}"); done; exec /usr/bin/mount "${a[@]}"
W
chmod 755 $T/tmp/man-nocompress/mount
efibootmgr > /tmp/boot-before.txt 2>/dev/null || true
chroot $T env PATH=/tmp/man-nocompress:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin $B request < /tmp/plan.json | python3 -c '
import json, sys
rc = 1
for line in sys.stdin:
    try: e = json.loads(line)
    except Exception: print(line, end=""); continue
    t = e.get("type")
    if t == "step": print("STEP", e["index"], e["title"], flush=True)
    elif t == "log": print("  ", e["line"], flush=True)
    elif t == "done": print("DONE", e["message"]); rc = 0
    elif t == "error": print("ERROR", e["message"], e.get("detail", ""))
sys.exit(rc)'

rm -rf $T/tmp/man-nocompress

echo "== remove the builder boot entry"
for n in $(efibootmgr 2>/dev/null | sed -n 's/^Boot\([0-9A-F]\{4\}\)\*\? MAN.*/\1/p'); do
  grep -q "^Boot$n" /tmp/boot-before.txt || efibootmgr -q -b $n -B
done

echo "== result"
blkid ${D}1 ${D}2 ${D}3
mkdir -p /mnt/esp /mnt/sys
mount ${D}1 /mnt/esp; find /mnt/esp -type f | head -20; umount /mnt/esp
mount -o subvolid=5 ${D}3 /mnt/sys; ls /mnt/sys; cat /mnt/sys/@system/etc/fstab
umount /mnt/sys
mount ${D}2 /mnt/sys; ls /mnt/sys; grep -E "menuentry|search" /mnt/sys/grub/grub.cfg | head -8; umount /mnt/sys
echo "APPLE INSTALL DONE"
