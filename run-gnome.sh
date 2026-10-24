#!/usr/bin/env bash
set -euo pipefail

mode=${1:-graphical}
project_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
rootfs_dir="$project_dir/lfs"
kernel="$rootfs_dir/boot/vmlinuz-7.3-rc7"
image="$rootfs_dir/man-gnome.img"
# Target disk for trying the installer. Sparse: grows only as it is written.
target="$rootfs_dir/install-target.qcow2"
target_size=40G
# UEFI firmware. The MAN installer requires UEFI, so the live image always
# boots through it. The variables file keeps the VM's own boot entries.
ovmf_code=/usr/share/OVMF/OVMF_CODE_4M.fd
ovmf_vars_template=/usr/share/OVMF/OVMF_VARS_4M.fd
ovmf_vars="$rootfs_dir/ovmf-vars.fd"

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
    echo "qemu-system-x86_64 is not installed or is not on PATH." >&2
    exit 1
fi

for file in "$kernel" "$image" "$ovmf_code" "$ovmf_vars_template"; do
    if [[ ! -f $file ]]; then
        echo "Missing boot file: $file" >&2
        exit 1
    fi
done
[[ -f $ovmf_vars ]] || cp "$ovmf_vars_template" "$ovmf_vars"

qemu_args=(
    -machine q35
    -device qemu-xhci,id=xhci
    -m 4096
    -smp 4
    -drive "if=pflash,format=raw,unit=0,readonly=on,file=$ovmf_code"
    -drive "if=pflash,format=raw,unit=1,file=$ovmf_vars"
)
# The live system boots like from a USB stick: firmware -> the stick's GRUB
# -> MAN kernel with the live image as root. The stick has boot priority, as
# a forgotten USB stick would; its menu starts an installed MAN by default
# when there is one, so restarting after installing boots the new system.
live_boot="$rootfs_dir/man-live-boot.img"
if [[ ! -f $live_boot || $kernel -nt $live_boot ]]; then
    "$project_dir/make-live-boot.sh" >&2
fi
live=(
    -drive "if=none,id=liveimg,file=$image,format=raw"
    -device virtio-blk-pci,drive=liveimg
    -drive "if=none,id=livestick,file=$live_boot,format=raw,readonly=on"
    -device usb-storage,drive=livestick,bootindex=1,removable=on
)
# Diagnosis without graphics: kernel started directly, console on serial.
live_direct=(
    -no-reboot
    -drive "file=$image,format=raw,if=virtio"
    -kernel "$kernel"
    -append "root=/dev/vda rootwait console=ttyS0,115200 rw loglevel=4"
)

if [[ -r /dev/kvm && -w /dev/kvm ]]; then
    qemu_args+=(-enable-kvm)
else
    echo "KVM is unavailable; using slower TCG emulation." >&2
    qemu_args+=(-machine accel=tcg)
fi

graphics=(
    -vga virtio
    -device usb-tablet
    -device virtio-keyboard-pci
    -serial stdio
)

case $mode in
    graphical)
        qemu_args+=("${live[@]}" -display gtk,show-cursor=on "${graphics[@]}")
        ;;
    sdl)
        qemu_args+=("${live[@]}" -display sdl,show-cursor=on "${graphics[@]}")
        ;;
    headless)
        qemu_args+=("${live_direct[@]}" -nographic)
        ;;
    install-test)
        # Live session plus an empty disk to install MAN onto.
        [[ -f $target ]] || qemu-img create -f qcow2 "$target" "$target_size" >/dev/null
        echo "Installer test disk: $target (appears as a $target_size Virtual Disk)." >&2
        # The test disk is in the firmware's boot list after the stick, like an
        # internal drive; that also makes its disk visible to the stick's GRUB.
        qemu_args+=("${live[@]}" -drive "if=none,id=target,file=$target,format=qcow2"
            -device virtio-blk-pci,drive=target,bootindex=2 -display gtk,show-cursor=on "${graphics[@]}")
        ;;
    installed)
        # Boot the system installed on the test disk, through its own GRUB.
        if [[ ! -f $target ]]; then
            echo "No installed test system yet; run '$0 install-test' first." >&2
            exit 1
        fi
        qemu_args+=(-drive "if=none,id=target,file=$target,format=qcow2"
            -device virtio-blk-pci,drive=target,bootindex=1 -display gtk,show-cursor=on "${graphics[@]}")
        # A USB stick with the update files from lfs/updates (see make-update-stick.sh).
        updates_stick="$rootfs_dir/updates-stick.img"
        if compgen -G "$rootfs_dir/updates/*.upd" >/dev/null; then
            newest=$(ls -t "$rootfs_dir"/updates/*.upd | head -n1)
            if [[ ! -f $updates_stick || $newest -nt $updates_stick ]]; then
                "$project_dir/make-update-stick.sh" >&2
            fi
        fi
        if [[ -f $updates_stick ]]; then
            echo "Updates USB stick attached: $(basename "$updates_stick")" >&2
            qemu_args+=(-drive "if=none,id=updstick,file=$updates_stick,format=raw,readonly=on"
                -device usb-storage,drive=updstick,removable=on)
        fi
        ;;
    iso)
        # The downloadable ISO (built by make-iso.sh) on a USB stick, plus the
        # installer test disk.
        iso=$(ls -t "$project_dir"/release/MAN-*.iso 2>/dev/null | head -n1 || true)
        if [[ -z $iso ]]; then
            echo "No ISO yet; build it with ./make-iso.sh first." >&2
            exit 1
        fi
        [[ -f $target ]] || qemu-img create -f qcow2 "$target" "$target_size" >/dev/null
        echo "Booting $(basename "$iso"); installer test disk: $target" >&2
        qemu_args+=(-drive "if=none,id=isostick,file=$iso,format=raw,readonly=on"
            -device usb-storage,drive=isostick,bootindex=1,removable=on
            -drive "if=none,id=target,file=$target,format=qcow2"
            -device virtio-blk-pci,drive=target,bootindex=2 -display gtk,show-cursor=on "${graphics[@]}")
        ;;
    *)
        echo "Usage: $0 [graphical|sdl|headless|install-test|installed|iso]" >&2
        exit 2
        ;;
esac

exec qemu-system-x86_64 "${qemu_args[@]}"
