#!/bin/bash
# Pack the Mac install disk (lfs/apple/disk.img, made by guest/apple-install.sh) into
# the OS package the Asahi installer downloads, plus its installer_data.json:
#   release/apple/MAN-<ver>-apple.zip   esp/ (GRUB + m1n1/boot.bin), boot.img, root.img, man.icns
#   release/apple/installer_data.json
# m1n1/boot.bin = Debian's m1n1 stage 2 + the kernel's Apple device trees + gzip(U-Boot),
# the same way update-m1n1 builds it on the installed Mac.
#   make-package.sh "10 TEST 6" https://github.com/MANCorporation/MAN/releases/download/v10-test6
set -euo pipefail
p=$(cd -- "$(dirname -- "$(readlink -f "$0")")/../.." && pwd)
VER=${1:?version, e.g. "10 TEST 6"}; URL=${2:?release download URL}
A=$p/lfs/apple; IMG=$A/disk.img; OUT=$p/release/apple
name="MAN-${VER// /-}-apple.zip"
W=$A/pkg; rm -rf $W; mkdir -p $W/esp $OUT
export PATH=$p/tools/mtools/usr/bin:$PATH

# MAN's "Erase Disk" layout: 1 EFI, 2 MAN_BOOT (ext4 /boot), 3 MAN (Btrfs).
read -r s1 n1 s2 n2 s3 n3 < <(/usr/sbin/sfdisk -J $IMG | python3 -c '
import json,sys; t=json.load(sys.stdin)["partitiontable"]; a=t["partitions"]
print(*[v for p in a[:3] for v in (p["start"], p["size"])])')
echo "ESP at $s1 (+$n1), boot at $s2 (+$n2), MAN at $s3 (+$n3) sectors"
# The Asahi installer only takes images whose size is a multiple of 4 KiB. The file
# systems end on a 4 KiB boundary, so the partial last block is unused.
n2=$((n2 / 8 * 8)); n3=$((n3 / 8 * 8))

# EFI partition files and FAT volume id (the Asahi installer formats a new FAT with
# this id, so the fstab UUID written by the MAN installer stays right).
mcopy -s -n -i "$IMG@@$((s1*512))" '::*' $W/esp/
volid=$(dd if=$IMG bs=1 skip=$((s1*512 + 0x43)) count=4 status=none | od -An -tx4 | tr -d ' ')
echo "ESP volume id: 0x$volid"
mkdir -p $W/esp/m1n1
m1=$A/debs/m1n1/usr/lib/m1n1/m1n1.bin
[ -f $m1 ] || { mkdir -p $A/debs/m1n1; dpkg-deb -x $A/debs/m1n1_*_arm64.deb $A/debs/m1n1; }
d=$A/kernel/dtbs/apple
{ cat $m1 $d/t6*.dtb $d/t81*.dtb; gzip -9 -n -c $A/ub/u-boot-nodtb.bin; } > $W/esp/m1n1/boot.bin
ls -la $W/esp/m1n1/boot.bin; find $W/esp -type f | sed "s#$W/##"

dd if=$IMG of=$W/boot.img bs=1M iflag=skip_bytes,count_bytes skip=$((s2*512)) count=$((n2*512)) conv=sparse status=none
dd if=$IMG of=$W/root.img bs=1M iflag=skip_bytes,count_bytes skip=$((s3*512)) count=$((n3*512)) conv=sparse status=none
cp $p/dev-tools/apple/man.icns $W/man.icns

rm -f $OUT/$name
# root.img with LZMA (the Asahi installer's Python reads it): about the size of the ISO.
python3 -I - $W $OUT/$name <<'PY'
import os, sys, zipfile
w, out = sys.argv[1], sys.argv[2]
with zipfile.ZipFile(out, "w", allowZip64=True) as z:
    for root, dirs, files in os.walk(os.path.join(w, "esp")):
        for f in sorted(files):
            path = os.path.join(root, f)
            z.write(path, os.path.relpath(path, w), zipfile.ZIP_DEFLATED)
    z.write(os.path.join(w, "man.icns"), "man.icns", zipfile.ZIP_DEFLATED)
    z.write(os.path.join(w, "boot.img"), "boot.img", zipfile.ZIP_LZMA)
    z.write(os.path.join(w, "root.img"), "root.img", zipfile.ZIP_LZMA)
PY
python3 -I - $OUT/$name <<'PY'
import sys, zipfile
z = zipfile.ZipFile(sys.argv[1])
for i in z.infolist():
    if i.filename.endswith(".img"):
        assert i.file_size % 4096 == 0, f"{i.filename}: {i.file_size} is not a multiple of 4 KiB"
        print(f"  {i.filename}: {i.file_size} bytes (4 KiB aligned), compressed {i.compress_size}")
print("  zip check:", z.testzip() or "all CRCs OK")
PY
ls -la $OUT/$name
size=$(stat -c %s $OUT/$name)
[ $size -lt $((2000*1024*1024)) ] || echo "WARNING: $name is over 2000 MiB (GitHub's limit is 2 GiB per file)"

# EFI: the Asahi installer makes a new 500 MB FAT partition and copies esp/ into it.
python3 - "$VER" "$URL/$name" "$volid" $((500*1024*1024)) $((n2*512)) $((n3*512)) > $OUT/installer_data.json <<'EOF'
import json, sys
ver, pkg, volid = sys.argv[1], sys.argv[2], sys.argv[3]
esp, boot, root = (int(v) for v in sys.argv[4:7])
print(json.dumps({"os_list": [{
    "name": f"MAN {ver}",
    "default_os_name": "MAN",
    "boot_object": "m1n1.bin",
    "next_object": "m1n1/boot.bin",
    "package": pkg,
    "icon": "man.icns",
    "supported_fw": ["13.5", "14.8.3"],
    "partitions": [
        {"name": "EFI", "type": "EFI", "size": f"{esp}B", "format": "fat", "volume_id": f"0x{volid}",
         "copy_firmware": True, "copy_installer_data": True, "source": "esp"},
        {"name": "Boot", "type": "Linux", "size": f"{boot}B", "image": "boot.img"},
        {"name": "MAN", "type": "Linux", "size": f"{root}B", "expand": True, "image": "root.img"},
    ],
}]}, indent=1))
EOF
cat $OUT/installer_data.json
(cd $OUT && sha256sum $name > $name.sha256)

# The macOS side: MAN's bootstrap script and a copy of the Asahi installer it runs
# (MIT licence), with installer/patch-urlcache.py applied: HTTP status checks and fresh
# signed links for downloads from GitHub releases.
IV=v0.9.2; MV=$IV-man1
mkdir -p $A/installer
[ -f $A/installer/installer-$IV.tar.gz ] || curl -sfL -o $A/installer/installer-$IV.tar.gz https://cdn.asahilinux.org/installer/installer-$IV.tar.gz
rm -rf $A/installer/tree; mkdir -p $A/installer/tree
tar -C $A/installer/tree -xzf $A/installer/installer-$IV.tar.gz
python3 -I $p/dev-tools/apple/installer/patch-urlcache.py $A/installer/tree/urlcache.py
tar -C $A/installer/tree --owner=0 --group=0 -czf $OUT/installer-$MV.tar.gz .
echo $MV > $OUT/latest
sed "s#@URL@#$URL#g" $p/dev-tools/apple/man-mac.sh.in > $OUT/man-mac.sh
ls -la $OUT
