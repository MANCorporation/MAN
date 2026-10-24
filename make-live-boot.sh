#!/usr/bin/env bash
# Build lfs/man-live-boot.img: the "USB stick" the live session boots from.
# It holds GRUB (UEFI) and the MAN kernel. Its menu starts an installed MAN
# by default when one is found on another disk, so a forgotten stick does
# not restart the live session after installing.
set -euo pipefail
project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
kernel="$project_dir/lfs/boot/vmlinuz-7.3-rc7"
out="$project_dir/lfs/man-live-boot.img"
# GRUB 2.14 (the version MAN installs) and mtools, kept in tools/.
grub_dir="$project_dir/tools/grub"
PATH="$grub_dir/usr/bin:$project_dir/tools/mtools/usr/bin:$PATH"
for tool in grub-mkstandalone mkfs.fat sfdisk mcopy mmd; do
    command -v "$tool" >/dev/null || [[ -x /sbin/$tool ]] || { echo "Missing tool: $tool" >&2; exit 1; }
done
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cat > "$work/grub.cfg" <<'CFG'
insmod part_gpt
insmod part_msdos
insmod fat
insmod btrfs
insmod ext2
insmod search_fs_file
insmod search_label
insmod all_video
insmod gfxterm
insmod png
# The full Unicode font (menu frame and arrows), from this stick.
loadfont $prefix/fonts/unicode.pf2
search --no-floppy --label MAN_LIVE --set=live
# An installed MAN: encrypted installs keep GRUB's files on the MAN_BOOT
# partition, plain installs inside the @system Btrfs subvolume.
set installed=
set installed_grub=
if search --no-floppy --label MAN_BOOT --set=installed; then
    set installed_grub=/grub
elif search --no-floppy --file /@system/boot/grub/grub.cfg --set=installed; then
    set installed_grub=/@system/boot/grub
fi
if [ -n "$installed" ]; then
    # Show the installed system's own boot menu (with its snapshot entries,
    # default entry and countdown); the live session is offered below it.
    # Modules keep loading from this stick ($prefix stays here): the
    # installed system's modules may belong to a different GRUB version.
    set root=$installed
    source ($installed)$installed_grub/grub.cfg
else
    set default=live
    set timeout=2
    set timeout_style=menu
    menuentry "UEFI Firmware Settings" --id uefi-firmware {
        fwsetup
    }
fi
menuentry "Try or Install MAN (live session)" --id live {
    echo "Starting the MAN live session..."
    linux ($live)/vmlinuz root=/dev/vda rootwait rw console=ttyS0,115200 loglevel=4
}
CFG

grub-mkstandalone -O x86_64-efi -d "$grub_dir/usr/lib/grub/x86_64-efi" -o "$work/BOOTX64.EFI" \
    --locales= --fonts= --themes= "boot/grub/grub.cfg=$work/grub.cfg" \
    "boot/grub/fonts/unicode.pf2=$grub_dir/usr/share/grub/unicode.pf2"

part_mib=250
/sbin/mkfs.fat -C -F 32 -n MAN_LIVE "$work/part.img" $((part_mib * 1024)) >/dev/null
mmd -i "$work/part.img" ::/EFI ::/EFI/BOOT
mcopy -i "$work/part.img" "$work/BOOTX64.EFI" ::/EFI/BOOT/BOOTX64.EFI
mcopy -i "$work/part.img" "$kernel" ::/vmlinuz

rm -f "$out.tmp"
truncate -s $((part_mib + 2))M "$out.tmp"
printf 'label: gpt\nstart=2048, size=%d, type=C12A7328-F81F-11D2-BA4B-00A0C93EC93B, name="MAN_LIVE"\n' \
    $((part_mib * 2048)) | /sbin/sfdisk -q "$out.tmp"
dd if="$work/part.img" of="$out.tmp" bs=1M seek=1 conv=notrunc status=none
mv "$out.tmp" "$out"
echo "Built $out"
