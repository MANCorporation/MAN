#!/usr/bin/env bash
# Build a UEFI El Torito ISO containing MAN as an initramfs.

set -euo pipefail

TARGET_DIR="$1"
IMAGES_DIR="$2"
ISO_PATH="$3"
ARCH="${4:-aarch64}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BOOTLOADER_DIR="$SCRIPT_DIR/bootloader"

case "$ARCH" in
    aarch64)
        KERNEL_NAME="Image"
        EFI_BOOT="BOOTAA64.EFI"
        SERIAL_CONSOLE="ttyAMA0,115200"
        ISO_VOLUME="MAN_AARCH64"
        ;;
    x86-64|x86_64)
        KERNEL_NAME="bzImage"
        [ -f "$IMAGES_DIR/$KERNEL_NAME" ] || KERNEL_NAME="vmlinuz"
        EFI_BOOT="BOOTX64.EFI"
        SERIAL_CONSOLE="ttyS0,115200"
        ISO_VOLUME="MAN_X86_64"
        ARCH="x86-64"
        ;;
    *)
        echo "ERROR: unsupported ISO architecture: $ARCH" >&2
        exit 2
        ;;
esac

find_tool() {
    name="$1"
    for directory in /Users/kristihack/.homebrew/bin /opt/homebrew/bin /usr/local/bin /usr/bin /bin; do
        [ -x "$directory/$name" ] && { printf '%s\n' "$directory/$name"; return 0; }
    done
    return 1
}

XORRISO="$(find_tool xorriso || true)"
MFORMAT="$(find_tool mformat || true)"
MCOPY="$(find_tool mcopy || true)"
MMD="$(find_tool mmd || true)"
CPIO="$(find_tool cpio || true)"
GZIP="$(find_tool gzip || true)"

[ -x "$XORRISO" ] || { echo "ERROR: xorriso is required (brew install xorriso)" >&2; exit 1; }
[ -x "$MFORMAT" ] && [ -x "$MCOPY" ] && [ -x "$MMD" ] || {
    echo "ERROR: mtools is required (brew install mtools)" >&2; exit 1;
}
[ -x "$CPIO" ] && [ -x "$GZIP" ] || { echo "ERROR: cpio and gzip are required" >&2; exit 1; }
[ -f "$IMAGES_DIR/$KERNEL_NAME" ] || { echo "ERROR: $ARCH kernel is missing" >&2; exit 1; }
[ -f "$BOOTLOADER_DIR/$EFI_BOOT" ] || { echo "ERROR: $ARCH MAN Boot Manager is missing" >&2; exit 1; }

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/man-uefi-iso.XXXXXX")"
trap 'rm -rf "$WORK_DIR"' EXIT
ISO_TREE="$WORK_DIR/iso"
ESP_IMAGE="$ISO_TREE/EFI/BOOT/efiboot.img"
INITRAMFS="$IMAGES_DIR/man-live-recovery.cpio.gz"
mkdir -p "$ISO_TREE/EFI/BOOT"

"$SCRIPT_DIR/create-recovery-image.sh" \
    "$TARGET_DIR" "$IMAGES_DIR/$KERNEL_NAME" "$BOOTLOADER_DIR" "$IMAGES_DIR" "$ARCH"

initramfs_bytes="$(stat -f%z "$INITRAMFS" 2>/dev/null || stat -c%s "$INITRAMFS")"
kernel_bytes="$(stat -f%z "$IMAGES_DIR/$KERNEL_NAME" 2>/dev/null || stat -c%s "$IMAGES_DIR/$KERNEL_NAME")"
esp_mb=$(( (initramfs_bytes + kernel_bytes + 96 * 1024 * 1024) / (1024 * 1024) + 1 ))

echo "  → Creating ${esp_mb} MiB EFI El Torito image..."
dd if=/dev/zero of="$ESP_IMAGE" bs=1m count=0 seek="$esp_mb" 2>/dev/null || \
    dd if=/dev/zero of="$ESP_IMAGE" bs=1M count=0 seek="$esp_mb" 2>/dev/null
"$MFORMAT" -i "$ESP_IMAGE" -F -v "MAN EFI" ::
"$MMD" -i "$ESP_IMAGE" ::/EFI ::/EFI/BOOT
"$MMD" -i "$ESP_IMAGE" ::/EFI/BOOT/icons
"$MMD" -i "$ESP_IMAGE" ::/EFI/BOOT/theme
"$MCOPY" -i "$ESP_IMAGE" "$BOOTLOADER_DIR/$EFI_BOOT" "::/EFI/BOOT/$EFI_BOOT"
"$MCOPY" -i "$ESP_IMAGE" "$IMAGES_DIR/$KERNEL_NAME" "::/EFI/BOOT/$KERNEL_NAME"
"$MCOPY" -i "$ESP_IMAGE" "$INITRAMFS" ::/EFI/BOOT/initramfs.cpio.gz
for icon in os_man.png manrecovery.png transparent.png arrow_left.png arrow_right.png; do
    [ ! -f "$BOOTLOADER_DIR/icons/$icon" ] || \
        "$MCOPY" -i "$ESP_IMAGE" "$BOOTLOADER_DIR/icons/$icon" ::/EFI/BOOT/icons/
done
"$MCOPY" -i "$ESP_IMAGE" "$BOOTLOADER_DIR/theme"/*.png ::/EFI/BOOT/theme/

# Generate the boot menu configuration from man-boot.conf (single source of truth).
# The ISO boots from initramfs (not a root partition), so we override the
# boot entries with live/recovery entries that use the initramfs.
# rEFInd on UEFI requires backslash paths — forward slashes cause broken icons.
BOOT_CONFIG="$WORK_DIR/boot.conf"

# Start with the visual configuration from man-boot.conf (banner, icons, menu layout, etc.)
# Then append ISO-specific boot entries.
{
    sed -n '1,/^# Boot entry: MAN Linux/p' "${BOOTLOADER_DIR}/man-boot.conf" \
        | sed '/^# Boot entry: MAN Linux/d'

    # ISO-specific boot entries: boot from initramfs (live/recovery environment)
    cat <<EOF
# ---------------------------------------------------------------------------
# Boot entry: MAN Utilities (live recovery environment)
# ---------------------------------------------------------------------------
menuentry "MAN Utilities" {
    loader \EFI\BOOT\\${KERNEL_NAME}
    initrd \EFI\BOOT\initramfs.cpio.gz
    options "console=tty0 console=${SERIAL_CONSOLE} rdinit=/sbin/init rw quiet loglevel=1 vt.global_cursor_default=0 net.ifnames=0 man.recovery=1"
    ostype Linux
    icon \EFI\BOOT\theme\man-system.png
}

# ---------------------------------------------------------------------------
# Boot entry: MAN Utilities (verbose — shows all kernel messages)
# ---------------------------------------------------------------------------
menuentry "MAN Utilities (Verbose)" {
    loader \EFI\BOOT\\${KERNEL_NAME}
    initrd \EFI\BOOT\initramfs.cpio.gz
    options "console=tty0 console=${SERIAL_CONSOLE} rdinit=/sbin/init rw loglevel=7 net.ifnames=0 man.recovery=1"
    ostype Linux
    icon \EFI\BOOT\theme\man-recovery.png
}
EOF
} > "$BOOT_CONFIG"

"$MCOPY" -i "$ESP_IMAGE" "$BOOT_CONFIG" ::/EFI/BOOT/man-os.conf
"$MCOPY" -i "$ESP_IMAGE" "$BOOT_CONFIG" ::/EFI/BOOT/refind.conf

echo "  → Writing UEFI-bootable ISO 9660 image..."
"$XORRISO" -as mkisofs -R -J -V "$ISO_VOLUME" \
    -eltorito-alt-boot -e EFI/BOOT/efiboot.img -no-emul-boot \
    -isohybrid-gpt-basdat -o "$ISO_PATH" "$ISO_TREE"

echo "  ✓ $ARCH UEFI ISO created: $ISO_PATH"
