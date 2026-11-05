#!/usr/bin/env bash
# Build the downloadable MAN live ISO from the OS image (no root needed).
#
#   ./make-iso.sh [IMAGE]          default IMAGE: lfs/man-gnome.img
#
# Output: release/MAN-<version>-amd64.iso (+ .sha256), or -arm64.iso for an
# ARM64 image (lfs/arm64/man-gnome-arm64.img; UEFI only, GRUB for arm64-efi
# and busybox from tools/arm64). The x86-64 ISO boots on UEFI and BIOS PCs, from a USB stick (write it with GNOME Disks "Restore Disk
# Image", dd or any image writer) or a DVD, and in QEMU:
#   ./run-gnome.sh iso
# Inside it: live/filesystem.squashfs (the image's system, zstd), the MAN
# kernel and a small busybox initramfs (iso/init) that puts a RAM overlay on
# top. The live session and the installer are exactly those of the image.
set -euo pipefail
project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
image=$(realpath "${1:-$project_dir/lfs/man-gnome.img}")
out_dir="$project_dir/release"
PATH="$project_dir/tools/mtools/usr/bin:$PATH"
fuse2fs="$project_dir/tools/fuse2fs/fuse2fs"
for tool in mksquashfs xorriso grub-mkrescue mformat cpio zstd fusermount3; do
    command -v "$tool" >/dev/null || { echo "Missing tool: $tool" >&2; exit 1; }
done
[[ -x $fuse2fs ]] || { echo "Missing tools/fuse2fs" >&2; exit 1; }
if pgrep -f -- "$(basename "$image")" | grep -qv "^$$\$" && pgrep -af qemu | grep -q -- "$(basename "$image")"; then
    echo "$image is in use by a running VM; shut it down first." >&2
    exit 1
fi

mkdir -p "$out_dir"
work=$(mktemp -d "$out_dir/.work.XXXXXX")
mnt="$work/mnt"
cleanup() {
    mountpoint -q "$mnt" 2>/dev/null && fusermount3 -u "$mnt"
    rm -rf "$work"
}
trap cleanup EXIT
mkdir -p "$mnt" "$work/iso/live" "$work/iso/boot/grub" "$work/initrd/bin"

echo "==> Opening $image (read-only)"
"$fuse2fs" "$image" "$mnt" -o ro,fakeroot >/dev/null ||
    { echo "Cannot open the image (was the VM shut down cleanly?)" >&2; exit 1; }

# The processor, from the ELF header of the image's shell (e_machine).
case $(od -An -tx1 -j18 -N2 "$mnt/usr/bin/bash" | tr -d ' \n') in
    3e00) arch=amd64 busybox="$project_dir/tools/busybox/busybox"
          grub_dir=/usr/lib/grub/x86_64-efi loader=grubx64.efi console="console=ttyS0,115200" ;;
    b700) arch=arm64 busybox="$project_dir/tools/arm64/root/usr/bin/busybox"
          grub_dir="$project_dir/tools/arm64/root/usr/lib/grub/arm64-efi" loader=grubaa64.efi console="console=ttyAMA0,115200" ;;
    *) echo "Unknown processor in $image" >&2; exit 1 ;;
esac
[[ -x $busybox && -d $grub_dir ]] || { echo "Missing $busybox or $grub_dir" >&2; exit 1; }
# /usr/lib/man/version: "10.9", or "10 TEST 5" for TEST builds (file name MAN-10-TEST-5-amd64.iso).
version=$(head -n1 "$mnt/usr/lib/man/version" 2>/dev/null | tr ' ' '-')
[[ -n $version ]] || version=$(sed -n 's/^VERSION_ID="\{0,1\}\([^"]*\)"\{0,1\}$/\1/p' "$mnt/etc/os-release")
version=${version:-10.0}
iso="$out_dir/MAN-$version-$arch.iso"
id="MAN-$version-$(date +%Y%m%d%H%M)-$(od -An -N3 -tx1 /dev/urandom | tr -d ' \n')"
kernel=$(ls "$mnt"/boot/vmlinuz-* | sort -V | tail -n1)
echo "    MAN $version, kernel $(basename "$kernel"), medium id $id"

echo "==> Compressing the system (this takes several minutes)"
# Same exclusions as an installed copy: nothing machine-specific or cached.
cat > "$work/exclude" <<'EOF'
tmp/*
var/tmp/*
var/log/journal/*
var/log/*.log
var/cache/apt/*.bin
var/cache/apt/archives/*.deb
var/lib/systemd/random-seed
etc/machine-id
var/lib/dbus/machine-id
root/.bash_history
root/.cache
home/*/.cache
home/*/.bash_history
lost+found
var/lib/clamav/*
EOF
# An empty machine-id makes every boot of the ISO get its own.
mksquashfs "$mnt" "$work/iso/live/filesystem.squashfs" -noappend -quiet -progress \
    -comp zstd -Xcompression-level 19 -b 1M -xattrs -xattrs-exclude "^system\." -wildcards -ef "$work/exclude" \
    -p 'etc/machine-id f 444 0 0 true'
# Uncompressed size, for the installer's free-space check.
df --output=used -B1 "$mnt" | tail -n1 | tr -d ' ' > "$work/iso/live/filesystem.size"
echo "$id" > "$work/iso/live/man-id"
cp "$kernel" "$work/iso/live/vmlinuz"

echo "==> Building the initramfs"
cp "$busybox" "$work/initrd/bin/busybox"
for app in sh mount umount mkdir cat cp sleep switch_root setsid cttyhack printf; do
    ln -s busybox "$work/initrd/bin/$app"
done
cp "$project_dir/iso/init" "$work/initrd/init"
chmod 755 "$work/initrd/init"
(cd "$work/initrd" && find . | LC_ALL=C sort | cpio -o -H newc -R 0:0 --quiet | zstd -19 -q) > "$work/iso/live/initrd.img"

echo "==> Writing the ISO"
sed "s/@ID@/$id/g; s/@CONSOLE@/$console/" "$project_dir/iso/grub.cfg" > "$work/iso/boot/grub/grub.cfg"
# For DUHACH! (installing from Windows): a GRUB that starts these files from
# a FAT partition on the hard disk. It finds the partition by live/man-id,
# then reads the same boot menu, modules and font as the ISO.
cat > "$work/partition-grub.cfg" <<'CFG'
search --no-floppy --file /live/man-id --set=root
set prefix=($root)/boot/grub
source ($root)/boot/grub/grub.cfg
CFG
mkdir -p "$work/iso/EFI/MAN-Installer"
grub-mkstandalone -d "$grub_dir" -O "$(basename "$grub_dir")" -o "$work/iso/EFI/MAN-Installer/$loader" --locales= --fonts= --themes= \
    --modules="part_gpt part_msdos fat iso9660 search search_fs_file configfile normal" \
    "boot/grub/grub.cfg=$work/partition-grub.cfg"
rm -f "$iso.tmp"
# x86-64: UEFI + BIOS (all platforms installed on this PC); ARM64: arm64-efi only.
rescue_dirs=(); [[ $arch == arm64 ]] && rescue_dirs=(-d "$grub_dir")
grub-mkrescue "${rescue_dirs[@]}" -o "$iso.tmp" --fonts=unicode --locales= --themes= \
    "$work/iso" -- -volid MAN_LIVE -joliet on -compliance iso_9660_level=3 2> "$work/mkrescue.log" ||
    { cat "$work/mkrescue.log" >&2; exit 1; }
mv "$iso.tmp" "$iso"
(cd "$out_dir" && sha256sum "$(basename "$iso")" > "$(basename "$iso").sha256")
echo "Built $iso ($(du -h "$iso" | cut -f1))"
