#!/bin/bash
# =============================================================================
# create-efi-disk.sh — Create a UEFI-bootable disk image for MAN Linux
#
# Creates a partitioned disk image:
#   Partition 1: FAT32 ESP (~100 MB) containing:
#     - \EFI\BOOT\BOOTAA64.EFI (or BOOTX64.EFI) — MAN Boot Manager
#     - \EFI\BOOT\Image (or vmlinuz) — the kernel (as EFI stub application)
#     - \EFI\BOOT\drivers_aa64\ext2_aa64.efi — ext2 driver (for root partition)
#     - EFI compatibility configuration used by the bundled application
#     - \EFI\BOOT\theme\ — MAN background and boot tiles
#   Partition 2: ext2 root filesystem — MAN rootfs
#
# Uses the kernel's built-in EFI stub (CONFIG_EFI_STUB=y) with the branded
# MAN Boot Manager providing normal and recovery entries.
#
# Tools required: sfdisk (Toolchain host), mtools (Homebrew), dd (system)
#
# Usage: create-efi-disk.sh <kernel> <rootfs> <output_dir> <arch>
# =============================================================================

set -e

KERNEL="$1"
ROOTFS="$2"
OUTPUT_DIR="$3"
ARCH="$4"

# --- Resolve arch-specific naming ---
case "$ARCH" in
    aarch64)
        EFI_BOOT="BOOTAA64.EFI"      # Default ARM64 EFI application
        KERNEL_NAME="Image"           # Kernel filename on ESP
        DRIVERS_DIR="drivers_aa64"
        SERIAL_CONSOLE="ttyAMA0,115200"
        ;;
    x86-64)
        EFI_BOOT="BOOTX64.EFI"
        KERNEL_NAME="vmlinuz"
        DRIVERS_DIR="drivers_x64"
        SERIAL_CONSOLE="ttyS0,115200"
        ;;
    *)
        echo "ERROR: Unknown architecture '$ARCH' (use 'aarch64' or 'x86-64')"
        exit 1
        ;;
esac

DISK_IMG="${OUTPUT_DIR}/man-${ARCH}-disk.img"
RECOVERY_IMAGE="${OUTPUT_DIR}/man-recovery.cpio.gz"
ESP_PART_SIZE_MB=100
if [ -f "$RECOVERY_IMAGE" ]; then
    RECOVERY_SIZE=$(stat -f%z "$RECOVERY_IMAGE" 2>/dev/null || stat -c%s "$RECOVERY_IMAGE")
    ESP_PART_SIZE_MB=$(( (RECOVERY_SIZE + 1024 * 1024 - 1) / (1024 * 1024) + 128 ))
    [ "$ESP_PART_SIZE_MB" -ge 512 ] || ESP_PART_SIZE_MB=512
fi

# Size the disk from the generated filesystem instead of assuming that a
# desktop image fits in a fixed 1 GiB container. Leave 16 MiB after rootfs for
# partition-table/alignment slack.
RFS_SIZE=$(stat -f%z "$ROOTFS" 2>/dev/null || stat -c%s "$ROOTFS" 2>/dev/null)
ROOTFS_SIZE_MB=$(( (RFS_SIZE + 1024 * 1024 - 1) / (1024 * 1024) ))
DISK_SIZE_MB=$((1 + ESP_PART_SIZE_MB + ROOTFS_SIZE_MB + 16))

# Partition layout (sectors, 512 bytes/sector)
ESP_START_SECTOR=2048
ESP_SIZE_SECTORS=$((ESP_PART_SIZE_MB * 1024 * 2))   # 100 MiB
ROOT_START_SECTOR=$((ESP_START_SECTOR + ESP_SIZE_SECTORS))

# Locate the bundled MAN Boot Manager assets.
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BOOTLOADER_DIR="${SCRIPT_DIR}/bootloader"

echo "=== Creating UEFI disk image for $ARCH (with MAN Boot Manager) ==="

[ -f "$KERNEL" ] || { echo "ERROR: Kernel not found at $KERNEL"; exit 1; }
[ -f "$ROOTFS" ] || { echo "ERROR: Rootfs not found at $ROOTFS"; exit 1; }
[ -f "${BOOTLOADER_DIR}/${EFI_BOOT}" ] || { echo "ERROR: MAN Boot Manager is missing at ${BOOTLOADER_DIR}/${EFI_BOOT}"; exit 1; }

# --- Step 1: Create blank disk image ---
echo "  → Creating ${DISK_SIZE_MB} MB disk image..."
rm -f "$DISK_IMG"
dd if=/dev/zero of="$DISK_IMG" bs=1M count=0 seek=$DISK_SIZE_MB 2>/dev/null

# --- Step 2: Partition (MBR: ESP + ext2 root) ---
echo "  → Partitioning (MBR: ESP + ext2 root)..."
sfdisk "$DISK_IMG" <<EOF
label: dos
unit: sectors
sector-size: 512

start=$ESP_START_SECTOR, size=$ESP_SIZE_SECTORS, type=EF, bootable
start=$ROOT_START_SECTOR, type=83
EOF

# --- Step 3: Create and format ESP (FAT32) using mtools ---
echo "  → Creating ESP (FAT32 via mtools)..."
ESP_IMG="${OUTPUT_DIR}/.man-${ARCH}-esp.tmp"
dd if=/dev/zero of="$ESP_IMG" bs=1M count=${ESP_PART_SIZE_MB} 2>/dev/null

# Format as FAT32
mformat -i "$ESP_IMG" -F -v "MAN ESP" ::

# Create directory structure (mtools mmd doesn't create parent dirs)
mmd -i "$ESP_IMG" ::/EFI
mmd -i "$ESP_IMG" ::/EFI/BOOT

# Install MAN Boot Manager as the default EFI application.
mcopy -i "$ESP_IMG" "${BOOTLOADER_DIR}/${EFI_BOOT}" ::/EFI/BOOT/${EFI_BOOT}
echo "    Installed MAN Boot Manager → /EFI/BOOT/${EFI_BOOT}"

# Copy the kernel (as EFI stub application)
mcopy -i "$ESP_IMG" "$KERNEL" ::/EFI/BOOT/${KERNEL_NAME}
echo "    Installed kernel → /EFI/BOOT/${KERNEL_NAME}"

if [ -f "$RECOVERY_IMAGE" ]; then
    mcopy -i "$ESP_IMG" "$RECOVERY_IMAGE" ::/EFI/BOOT/man-recovery.cpio.gz
    echo "    Installed MAN Recovery → /EFI/BOOT/man-recovery.cpio.gz"
fi

# Copy the ext2 driver so the boot manager can inspect the root partition.
if [ -d "${BOOTLOADER_DIR}/${DRIVERS_DIR}" ]; then
    mmd -i "$ESP_IMG" ::/EFI/BOOT/${DRIVERS_DIR}
    mcopy -i "$ESP_IMG" "${BOOTLOADER_DIR}/${DRIVERS_DIR}"/*.efi ::/EFI/BOOT/${DRIVERS_DIR}/ 2>/dev/null || true
    echo "    Installed ext2 driver → /EFI/BOOT/${DRIVERS_DIR}/"
fi

# Keep only MAN's compatibility icons for firmware text/graphics fallbacks.
if [ -d "${BOOTLOADER_DIR}/icons" ]; then
    mmd -i "$ESP_IMG" ::/EFI/BOOT/icons
    for icon in os_man.png manrecovery.png transparent.png arrow_left.png arrow_right.png; do
        [ ! -f "${BOOTLOADER_DIR}/icons/${icon}" ] || \
            mcopy -i "$ESP_IMG" "${BOOTLOADER_DIR}/icons/${icon}" ::/EFI/BOOT/icons/
    done
fi

# Install the full-screen MAN background and large boot-option tiles.
mmd -i "$ESP_IMG" ::/EFI/BOOT/theme
mcopy -i "$ESP_IMG" "${BOOTLOADER_DIR}/theme"/*.png ::/EFI/BOOT/theme/
echo "    Installed MAN boot theme → /EFI/BOOT/theme/"

# Give the development root filesystem its conventional MAN label.
# The rootfs image from buildroot has no label by default.
# e2label/tune2fs from the Toolchain are Linux binaries and cannot run on macOS,
# so we write the ext2 volume name directly into the superblock.
# The ext2 superblock begins at byte 1024; the s_volume_name field is at
# offset 0x7E (126) within the superblock, i.e. byte 1150 in the file.
python3 -c "
import sys
with open(sys.argv[1], 'r+b') as f:
    f.seek(1150)
    f.write(b'rootfs' + b'\x00' * 10)
" "$ROOTFS"

# Generate the boot menu configuration from man-boot.conf (single source of truth).
# Substitute arch-specific placeholders.
# This image is launched as a VirtIO disk.  Unlike LABEL=, /dev/vda2 is
# resolvable by the kernel without an initramfs.
ROOT_DEV="/dev/vda2"
BOOT_CONFIG="/tmp/man-boot.conf.$$"
sed \
    -e "s|@KERNEL_NAME@|${KERNEL_NAME}|g" \
    -e "s|@SERIAL_CONSOLE@|${SERIAL_CONSOLE}|g" \
    -e "s|@ROOT_DEVICE@|${ROOT_DEV}|g" \
    "${BOOTLOADER_DIR}/man-boot.conf" > "$BOOT_CONFIG"

mcopy -i "$ESP_IMG" "$BOOT_CONFIG" ::/EFI/BOOT/man-os.conf
mcopy -i "$ESP_IMG" "$BOOT_CONFIG" ::/EFI/BOOT/refind.conf
rm -f "$BOOT_CONFIG"

# --- Step 4: Write ESP to disk image at partition offset ---
echo "  → Writing ESP to disk image..."
dd if="$ESP_IMG" of="$DISK_IMG" bs=512 seek=$ESP_START_SECTOR conv=notrunc 2>/dev/null
rm -f "$ESP_IMG"

# --- Step 5: Write rootfs to disk image at partition offset ---
echo "  → Writing rootfs to root partition..."
BSECTORS=$(( (RFS_SIZE + 511) / 512 ))
dd if="$ROOTFS" of="$DISK_IMG" bs=512 seek=$ROOT_START_SECTOR count=$BSECTORS conv=notrunc 2>/dev/null

echo ""
echo "  ✓ UEFI disk image created (with MAN Boot Manager): $DISK_IMG"
ls -la "$DISK_IMG"
file "$DISK_IMG"
