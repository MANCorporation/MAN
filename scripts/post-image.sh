#!/bin/bash
# Post-image script for MAN Linux Distribution.
# Called by Toolchain after images have been generated.
#
# $1 = target directory
# $2 = images directory

set -e

# Buildroot supplies TARGET_DIR/BINARIES_DIR as environment variables. Keep
# positional arguments for direct invocation, but never let an empty argument
# replace a populated Buildroot variable.
TARGET_DIR="${TARGET_DIR:-${1:-}}"
IMAGES_DIR="${BINARIES_DIR:-${2:-}}"

[ -n "$TARGET_DIR" ] || { echo "ERROR: TARGET_DIR is not set" >&2; exit 1; }
[ -n "$IMAGES_DIR" ] || { echo "ERROR: BINARIES_DIR/IMAGES_DIR is not set" >&2; exit 1; }
mkdir -p "$IMAGES_DIR"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "=== MAN post-image script ==="

# Buildroot's fakeroot flow is not available on macOS. Its generated ext2
# image can contain metadata that the Linux ext4 driver rejects even though
# the host e2fsck reports it clean. Recreate it directly from the finalized
# tree with the small, QEMU-tested feature set used by MAN.
if [ "$(uname -s)" = "Darwin" ] && [ -x "${HOST_DIR:-}/sbin/mkfs.ext2" ]; then
    echo "  → Rebuilding macOS rootfs with Linux-compatible ext2 metadata..."
    # Buildroot normally applies users only inside its fakeroot copy. The
    # macOS compatibility repack below reads TARGET_DIR directly, so mirror
    # the generated account database there first or service users such as
    # cosmic-greeter disappear from the bootable disk.
    users_table="${BUILD_DIR:-$(dirname "$TARGET_DIR")/build}/buildroot-fs/full_users_table.txt"
    mkusers="$SCRIPT_DIR/../toolchain/support/scripts/mkusers"
    if [ -f "$users_table" ] && [ -x "$mkusers" ]; then
        BR2_CONFIG="${BR2_CONFIG:-$(dirname "$TARGET_DIR")/.config}" \
            "$mkusers" "$users_table" "$TARGET_DIR" >/dev/null
    fi
    DYLD_LIBRARY_PATH="${HOST_DIR}/lib:${HOST_DIR}/lib64:${HOST_DIR}/usr/lib" \
        "${HOST_DIR}/sbin/mkfs.ext2" -F -d "$TARGET_DIR" -r 1 -N 0 -m 5 \
        -L rootfs -I 256 -O '^64bit,^resize_inode,^dir_index' \
        "$IMAGES_DIR/rootfs.ext2" "${MAN_ROOTFS_SIZE:-8192M}"
    # The host e2fsprogs build occasionally leaves its allocation bitmap one
    # block behind after populating a large image on APFS. Repair it before the
    # image is embedded in the UEFI disk, rather than shipping a filesystem
    # that Linux may mount with an error.
    DYLD_LIBRARY_PATH="${HOST_DIR}/lib:${HOST_DIR}/lib64:${HOST_DIR}/usr/lib" \
        "${HOST_DIR}/sbin/e2fsck" -fy "$IMAGES_DIR/rootfs.ext2"
fi

# Create a MAN-specific manifest
MANIFEST="${IMAGES_DIR}/MANIFEST"

cat > "${MANIFEST}" << 'MANIFEST_EOF'
MAN Linux Distribution v0.1.0
============================
Architectures: x86_64, aarch64
Core utilities: uutils/coreutils (Rust)
C library: musl (x86_64) / glibc (aarch64)
Default shell: bash + zsh
Kernel: Linux 6.10.8
Build system: Toolchain
MANIFEST_EOF

# List image files
echo "" >> "${MANIFEST}"
echo "Generated images:" >> "${MANIFEST}"
for f in "${IMAGES_DIR}"/*; do
    if [ -f "$f" ]; then
        SIZE=$(du -h "$f" | cut -f1)
        echo "  $(basename "$f") — ${SIZE}" >> "${MANIFEST}"
    fi
done

echo "  ✓ MANIFEST written to ${MANIFEST}"

# --- UEFI disk image creation ---
# Create a partitioned, UEFI-bootable disk image (.img) with:
#   - Partition 1: FAT32 ESP with kernel as EFI stub (BOOTAA64.EFI / BOOTX64.EFI)
#   - Partition 2: ext2 root filesystem
# This requires mtools for FAT manipulation. If unavailable, skip gracefully.
CREATE_EFI_SCRIPT="${SCRIPT_DIR}/create-efi-disk.sh"

# Detect architecture from available kernel image
if [ -f "${IMAGES_DIR}/Image" ]; then
    DISK_KERNEL="${IMAGES_DIR}/Image"
    DISK_ARCH="aarch64"
elif [ -f "${IMAGES_DIR}/bzImage" ]; then
    DISK_KERNEL="${IMAGES_DIR}/bzImage"
    DISK_ARCH="x86-64"
elif [ -f "${IMAGES_DIR}/vmlinuz" ]; then
    DISK_KERNEL="${IMAGES_DIR}/vmlinuz"
    DISK_ARCH="x86-64"
else
    DISK_KERNEL=""
fi

if [ -n "$DISK_KERNEL" ] && [ -f "${IMAGES_DIR}/rootfs.ext2" ] && [ -f "$CREATE_EFI_SCRIPT" ]; then
    # Ensure sfdisk (Toolchain host tools) is on PATH
    if [ -n "${HOST_DIR:-}" ]; then
        export PATH="${HOST_DIR}/sbin:${HOST_DIR}/bin:${PATH}"
    fi

    # Find mtools (Homebrew on macOS, or system)
    MTOOLS_FOUND=0
    for p in "${HOME}/.homebrew/bin" "/opt/homebrew/bin" "/usr/local/bin" "/usr/bin"; do
        if [ -x "${p}/mformat" ]; then
            export PATH="${p}:${PATH}"
            MTOOLS_FOUND=1
            break
        fi
    done

    if [ "$MTOOLS_FOUND" -eq 1 ]; then
        if [ -x "${SCRIPT_DIR}/create-recovery-image.sh" ]; then
            echo ""
            echo "  → Building the independent MAN Recovery image..."
            "${SCRIPT_DIR}/create-recovery-image.sh" \
                "$TARGET_DIR" "$DISK_KERNEL" "${SCRIPT_DIR}/bootloader" "$IMAGES_DIR" "$DISK_ARCH"
        fi
        echo ""
        echo "  → Creating UEFI disk image for ${DISK_ARCH}..."
        bash "$CREATE_EFI_SCRIPT" "$DISK_KERNEL" "${IMAGES_DIR}/rootfs.ext2" "$IMAGES_DIR" "$DISK_ARCH" 2>&1 || \
            echo "  ⚠ UEFI disk image creation failed (continue anyway)"
    else
        echo "  ℹ mtools not found — skipping UEFI disk image"
        echo "    (Install via: brew install mtools, then rerun: man disk --arch ${DISK_ARCH})"
    fi
fi

echo "=== MAN post-image complete ==="
