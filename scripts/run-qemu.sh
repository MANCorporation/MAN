#!/bin/bash
# run-qemu.sh — boot a MAN distribution image in QEMU.
# Usage: ./run-qemu.sh [x86_64|aarch64] [memory_mb] [cpus]
#
# This script mirrors the QEMU invocation used by `man test` and applies the
# same performance tuning: hardware acceleration (HVF/KVM), host CPU
# passthrough, virtio-blk writeback caching, VirtIO GPU + balloon, and a
# graphical display on macOS hosts.

set -e

ARCH="${1:-auto}"
MEMORY="${2:-4096}"
CPUS="${3:-4}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Auto-detect architecture if not specified
if [ "$ARCH" = "auto" ]; then
    ARCH=$(uname -m)
    case "$ARCH" in
        x86_64|amd64) ARCH="x86_64" ;;
        aarch64|arm64) ARCH="aarch64" ;;
        *) echo "Unknown architecture: $ARCH"; exit 1 ;;
    esac
fi

IMAGES_DIR="${PROJECT_ROOT}/output/${ARCH}/images"

if [ ! -d "$IMAGES_DIR" ]; then
    echo "No build found at $IMAGES_DIR"
    echo "Run: man build --arch $ARCH"
    exit 1
fi

echo "Booting MAN ($ARCH) in QEMU..."
echo "  Images: $IMAGES_DIR"
echo "  Memory: ${MEMORY}MB  CPUs: ${CPUS}"
echo ""

# Disk I/O options shared across all virtio drives.
# cache=writeback gives the best single-vCPU throughput for VM-local storage;
# discard=unmap lets the guest free blocks on the sparse image, keeping the
# file size bounded.
DISK_OPTS="format=raw,if=virtio,cache=writeback,discard=unmap"

case "$ARCH" in
    x86_64)
        # Ensure Homebrew tools are first on PATH.
        export PATH="$HOME/.homebrew/bin:$HOME/.homebrew/sbin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin"

        UEFI_DISK="${IMAGES_DIR}/man-x86-64-disk.img"
        DISK_IMG="${IMAGES_DIR}/rootfs.ext2"
        OVMF_FW="${HOME}/.homebrew/share/qemu/edk2-x86_64-code.fd"
        [ -f "$OVMF_FW" ] || OVMF_FW="/opt/homebrew/share/qemu/edk2-x86_64-code.fd"

        # Prefer the UEFI disk image (two-partition: ESP + ext2 rootfs),
        # fall back to raw rootfs for quick headless boots.
        if [ -f "$UEFI_DISK" ]; then
            exec qemu-system-x86_64 \
                -machine q35,accel=hvf \
                -cpu host \
                -m "$MEMORY" \
                -smp "$CPUS" \
                -drive "if=pflash,format=raw,readonly=on,file=${OVMF_FW}" \
                -drive "file=${UEFI_DISK},${DISK_OPTS}" \
                -device qemu-xhci,id=usb \
                -device usb-kbd \
                -device usb-tablet \
                -device virtio-vga,id=gpu \
                -device virtio-balloon-pci \
                -device e1000,netdev=net0 \
                -netdev user,id=net0,net=10.0.2.0/24,hostfwd=tcp::2223-:22 \
                -serial file:/tmp/qemu-x86_64-serial.log \
                -monitor tcp:127.0.0.1:4444,server,nowait \
                -display cocoa
        elif [ -f "$DISK_IMG" ]; then
            exec qemu-system-x86_64 \
                -machine q35,accel=hvf \
                -cpu host \
                -m "$MEMORY" \
                -smp "$CPUS" \
                -drive "file=${DISK_IMG},format=raw" \
                -device virtio-vga,id=gpu \
                -device virtio-balloon-pci \
                -serial mon:stdio \
                -display cocoa \
                -append "console=ttyS0 root=/dev/sda rw"
        else
            echo "No bootable image found in $IMAGES_DIR"
            ls "$IMAGES_DIR"
            exit 1
        fi
        ;;

    aarch64)
        # Ensure Homebrew tools are in PATH (QEMU + util-linux for sfdisk)
        export PATH="$HOME/.homebrew/bin:$HOME/.homebrew/sbin:/opt/homebrew/bin:$PATH"
        DISK_IMG="${IMAGES_DIR}/man-aarch64-disk.img"
        if [ ! -f "$DISK_IMG" ]; then
            echo "Disk image not found: $DISK_IMG"
            echo "Run: man build --arch aarch64"
            exit 1
        fi
        exec qemu-system-aarch64 \
            -machine virt,accel=hvf \
            -cpu host,pmu=off \
            -m "$MEMORY" \
            -smp "$CPUS" \
            -bios "$HOME/.homebrew/share/qemu/edk2-aarch64-code.fd" \
            -device qemu-xhci,id=usb \
            -device usb-kbd \
            -device usb-tablet \
            -device virtio-gpu-pci,id=gpu \
            -device ramfb \
            -device virtio-balloon-pci \
            -device e1000,netdev=net0 \
            -netdev user,id=net0,net=10.0.2.0/24,hostfwd=tcp::2222-:22 \
            -drive "file=${DISK_IMG},${DISK_OPTS}" \
            -serial file:/tmp/qemu-serial.log \
            -monitor tcp:127.0.0.1:4444,server,nowait \
            -display cocoa
        ;;

    *)
        echo "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac
