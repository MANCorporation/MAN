#!/bin/bash
# Quick look at MAN's boot menu without installing anything: builds a test ESP
# (MAN's patched GRUB, the theme, grub.cfg from 06/49_man_bootmenu with sample
# entries in between) plus a fake USB stick, and starts it in QEMU (UEFI).
#   test-vm.sh [DIR]   (default /tmp/claude-1000/gt; QEMU monitor: DIR/mon.sock)
# Hold Alt: dev-tools/vm/hold.py DIR/mon.sock alt 15 right after start.
set -euo pipefail
p=$(cd "$(dirname "$0")/../.." && pwd)
T=${1:-/tmp/claude-1000/gt}
G=$p/man-installer/data/grub
O=$p/dev-tools/grub-src/build/obj-efi/grub-core
mkdir -p "$T"; rm -rf "$T/esp" "$T/usb"
mkdir -p "$T/esp/EFI/BOOT" "$T/esp/boot/grub/themes" "$T/esp/boot/grub/x86_64-efi" "$T/usb/EFI/BOOT"
cp -r "$G/theme/man" "$T/esp/boot/grub/themes/man"
cp "$O"/*.mod "$O"/*.lst "$T/esp/boot/grub/x86_64-efi/"
mkdir -p "$T/esp/boot/grub/fonts"; cp /usr/share/grub/unicode.pf2 "$T/esp/boot/grub/fonts/"
grub-mkimage -d "$O" -O x86_64-efi -p '(hd0,msdos1)/boot/grub' -o "$T/esp/EFI/BOOT/BOOTX64.EFI" part_msdos part_gpt fat normal configfile
# The fake USB stick: a FAT image with its own serial (QEMU's fat: drives all
# share one), partitioned like a real stick.
mt=$p/tools/mtools/usr/bin
rm -f "$T/usb.img"; truncate -s 64M "$T/usb.img"
printf 'label: dos\nstart=2048, type=c\n' | /sbin/sfdisk -q "$T/usb.img"
"$mt/mformat" -i "$T/usb.img@@1M" -N 1234ABCD -v TESTSTICK ::
"$mt/mmd" -i "$T/usb.img@@1M" ::/EFI ::/EFI/BOOT
"$mt/mcopy" -i "$T/usb.img@@1M" "$T/esp/EFI/BOOT/BOOTX64.EFI" ::/EFI/BOOT/BOOTX64.EFI
# 06 without its build-time parts (grub-probe, /boot, the EFI partition).
sed -e 's#^\. "\${pkgdatadir.*#grub_probe=true; prepare_grub_to_access_device() { :; }; make_system_path_relative_to_its_root() { echo /boot/grub/themes/man; }#' \
    -e 's#^src=.*#src=/x#; s#^\[ -f "\$src/theme.txt" \] || exit 0##; s#^cp -ru.*##; s#^mkdir -p "\$dst"##; s#^\[ -d "\$dst/icons" \] || exit 0##' \
    -e 's#^if mountpoint -q /boot/efi.*#env_uuid=5678-EF01; env_rel=/EFI/MAN/bootmenu.env; if false; then#' "$G/06_man_bootmenu" > "$T/06.sh"
{
cat <<'CFG'
set root=(hd0,msdos1)
insmod all_video
insmod gfxterm
loadfont unicode
set gfxmode=auto
terminal_output gfxterm
set menuentry_id_option=--id
export menuentry_id_option
CFG
GRUB_TIMEOUT=5 sh "$T/06.sh"
# Ctrl+Enter's file: on the test ESP.
cat <<'CFG'
menuentry 'MAN GNU/Linux' --class gnu-linux --class gnu --class os --id gnulinux-simple-x { echo "BOOT: MAN (menu_mod=$menu_mod)"; sleep 30 }
submenu 'Advanced options for MAN GNU/Linux' --id gnulinux-advanced-x { menuentry 'MAN, with Linux 7.3' --class gnu-linux { echo k; sleep 30 } }
menuentry 'Windows 11' --class windows11 --class windows --class os --id man-windows { echo "BOOT: Windows (menu_mod=$menu_mod)"; sleep 30 }
menuentry 'Install MAN update 10 TEST 7' --class update --class recovery --id man-update { echo upd; sleep 30 }
submenu 'Restore a MAN snapshot' --class snapshot --class recovery --id man-snapshots { menuentry 'Before update -- 2026-11-17' --class snapshot { echo s; sleep 30 } }
menuentry 'MAN Recovery' --class man-recovery --class recovery --id man-recovery { echo "BOOT: Recovery"; sleep 30 }
menuentry 'macOS Golden Gate' --class macos-golden-gate --class macos --id man-macos { echo "BOOT: macOS"; sleep 30 }
menuentry 'UEFI Firmware Settings' --class firmware --id uefi-firmware { fwsetup }
CFG
# 49 with sample answers.
awk '/^cat <<.EOF.$/ { if (!n++) { p = 1; next } } p && /^EOF$/ { exit } p' "$G/49_man_bootmenu"
cat <<'CFG'
menuentry 'Windows 11' --class windows11 --class windows --id man-windows { man_start man-windows; configfile "$man_cfg" }
menuentry 'macOS Golden Gate' --class macos-golden-gate --class macos --id man-macos { man_start man-macos; configfile "$man_cfg" }
man_scan_usb
menuentry 'MAN Recovery' --class man-recovery --id man-recovery { man_start man-recovery; configfile "$man_cfg" }
fi
CFG
} > "$T/esp/boot/grub/grub.cfg"
if [ -n "${DEBUG:-}" ]; then
  sed -i 's/^terminal_output gfxterm$/terminal_output gfxterm serial/' "$T/esp/boot/grub/grub.cfg"
  sed -i '1i insmod serial\nserial --unit=0 --speed=115200\nterminal_output --append serial\nterminal_input --append serial' "$T/esp/boot/grub/grub.cfg"
  sed -i 's/^  if man_connect; then$/  echo "DBG before pu=$man_pu pd=$man_pd pp=$man_pp ru=$man_ru tu=$man_tu eu=$man_eu"; ls\n  if man_connect; then\n    echo DBG connected; ls/' "$T/esp/boot/grub/grub.cfg"
  sed -i 's/^function man_start {$/function man_start {\n  echo "DBG start $1 mod=$menu_mod env=$man_env"; sleep 3/' "$T/esp/boot/grub/grub.cfg"
  sed -i 's/^function man_scan_usb {$/function man_scan_usb {\n  echo DBG scan/' "$T/esp/boot/grub/grub.cfg"
fi
mkdir -p "$T/esp/EFI/MAN"; grub-editenv "$T/esp/EFI/MAN/bootmenu.env" create
# The ESP as a real FAT image too (QEMU's fat: drive breaks when GRUB rescans).
rm -f "$T/esp.img"; truncate -s 96M "$T/esp.img"
printf 'label: dos\nstart=2048, type=ef\n' | /sbin/sfdisk -q "$T/esp.img"
"$mt/mformat" -i "$T/esp.img@@1M" -N 5678EF01 -v MANESP ::
"$mt/mcopy" -s -i "$T/esp.img@@1M" "$T/esp/EFI" "$T/esp/boot" ::/
[ -f "$T/vars.fd" ] || cp /usr/share/OVMF/OVMF_VARS_4M.fd "$T/vars.fd"
[ -n "${NOCONNECT:-}" ] && sed -i "s/^  man_connect_all/  true/" "$T/esp/boot/grub/grub.cfg" && "$mt/mcopy" -o -i "$T/esp.img@@1M" "$T/esp/boot/grub/grub.cfg" ::/boot/grub/grub.cfg
exec qemu-system-x86_64 -machine q35 -enable-kvm -m 1024 -S \
  -drive if=pflash,format=raw,unit=0,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,unit=1,file="$T/vars.fd" \
  -drive file="$T/esp.img",format=raw,if=none,id=d0 -device virtio-blk-pci,drive=d0,bootindex=0 \
  -drive file="$T/usb.img",format=raw,if=none,id=u -device qemu-xhci -device usb-storage,drive=u \
  -debugcon file:"$T/debug.log" -global isa-debugcon.iobase=0x402 -serial file:"$T/serial.log" -vga std -display none -monitor unix:"$T/mon.sock",server,nowait
