#!/usr/bin/env bash
# Build MAN Recovery as a self-contained initramfs plus a pristine MAN payload.

set -euo pipefail

TARGET_DIR="$1"
KERNEL="$2"
BOOTLOADER_DIR="$3"
OUTPUT_DIR="$4"
ARCH="${5:-aarch64}"
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BOOT_CONFIG_TEMPLATE="$PROJECT_ROOT/scripts/bootloader/man-boot.conf"
OVERLAY_DIR="$PROJECT_ROOT/overlay"

case "$ARCH" in
    aarch64)
        EFI_BOOT="BOOTAA64.EFI"
        KERNEL_NAME="Image"
        RUST_TRIPLE="aarch64-unknown-linux-gnu"
        OUTPUT_ARCH="aarch64"
        ;;
    x86-64|x86_64)
        EFI_BOOT="BOOTX64.EFI"
        KERNEL_NAME="vmlinuz"
        RUST_TRIPLE="x86_64-unknown-linux-gnu"
        OUTPUT_ARCH="x86_64"
        ;;
    *)
        echo "ERROR: unsupported recovery architecture: $ARCH" >&2
        exit 2
        ;;
esac

UTILITIES_BINARY="$PROJECT_ROOT/support/man-utilities/target/$RUST_TRIPLE/release/man-utilities"
SEATD_BINARY="$PROJECT_ROOT/output/$OUTPUT_ARCH/build/seatd-0.8.0/build/seatd"
LIBSEAT_LIBRARY="$PROJECT_ROOT/output/$OUTPUT_ARCH/build/seatd-0.8.0/build/libseat.so.1"
AGREETY_BINARY="$PROJECT_ROOT/output/$OUTPUT_ARCH/build/greetd/target/$RUST_TRIPLE/release/agreety"

find_tool() {
    local name="$1"
    local directory
    for directory in /Users/kristihack/.homebrew/bin /opt/homebrew/bin /usr/local/bin /usr/bin /bin; do
        if [ -x "$directory/$name" ]; then
            printf '%s\n' "$directory/$name"
            return 0
        fi
    done
    return 1
}

CPIO="$(find_tool cpio || true)"
GZIP="$(find_tool gzip || true)"
[ -x "$CPIO" ] && [ -x "$GZIP" ] || {
    echo "ERROR: cpio and gzip are required to build MAN Recovery" >&2
    exit 1
}
[ -f "$KERNEL" ] || { echo "ERROR: recovery kernel is missing: $KERNEL" >&2; exit 1; }
[ -f "$BOOTLOADER_DIR/$EFI_BOOT" ] || { echo "ERROR: MAN Boot Manager is missing" >&2; exit 1; }
[ -f "$BOOT_CONFIG_TEMPLATE" ] || { echo "ERROR: MAN boot configuration template is missing" >&2; exit 1; }
[ -d "$OVERLAY_DIR" ] || { echo "ERROR: MAN runtime overlay is missing" >&2; exit 1; }
python3 "$PROJECT_ROOT/scripts/validate-i18n.py"

work_dir="$(mktemp -d "${TMPDIR:-/tmp}/man-recovery.XXXXXX")"
trap 'rm -rf "$work_dir"' EXIT
system_tree="$work_dir/system"
recovery_tree="$work_dir/recovery"
system_archive="$work_dir/man-system.cpio.gz"
inner_image="$OUTPUT_DIR/man-recovery.cpio.gz"
live_image="$OUTPUT_DIR/man-live-recovery.cpio.gz"

install_legal_docs() {
    local tree="$1"
    local docs="$tree/usr/share/doc/MAN"
    mkdir -p "$docs"
    install -m 0644 "$PROJECT_ROOT/LICENSE" "$docs/LICENSE"
    install -m 0644 "$PROJECT_ROOT/LICENSES/LicenseRef-MAN-Proprietary.txt" \
        "$docs/MAN-PROPRIETARY-LICENSE.txt"
    install -m 0644 "$PROJECT_ROOT/THIRD_PARTY_NOTICES.md" \
        "$docs/THIRD_PARTY_NOTICES.md"
    install -m 0644 "$PROJECT_ROOT/DISTRIBUTION_COMPLIANCE.md" \
        "$docs/DISTRIBUTION_COMPLIANCE.md"
    install -m 0644 "$PROJECT_ROOT/ASSET_PROVENANCE.md" \
        "$docs/ASSET_PROVENANCE.md"
    install -m 0644 "$PROJECT_ROOT/scripts/bootloader/THIRD_PARTY-ICON-LICENSES.md" \
        "$docs/rEFInd-ICON-NOTICES.md"
    install -m 0644 "$PROJECT_ROOT/support/pam-client/LICENSE" \
        "$docs/MPL-2.0-pam-client.txt"
    install -m 0644 "$PROJECT_ROOT/toolchain/COPYING" \
        "$docs/GPL-2.0-or-later.txt"
    install -m 0644 "$PROJECT_ROOT/LICENSES/MIT.txt" \
        "$docs/Haiku-ICONS-MIT.txt"
    # COSMIC and rEFInd are GPLv3-covered components. The GPL text is
    # identical for GPL-3.0-only and GPL-3.0-or-later; the component mapping
    # in THIRD_PARTY_NOTICES.md records which term applies to each component.
    for gpl3_source in \
        "$PROJECT_ROOT/output/$OUTPUT_ARCH/build/cosmic-epoch/cosmic-comp/LICENSE" \
        "$PROJECT_ROOT/output/$OUTPUT_ARCH/build/cosmic-epoch/cosmic-greeter/LICENSE"; do
        if [ -f "$gpl3_source" ]; then
            install -m 0644 "$gpl3_source" "$docs/GPL-3.0.txt"
            break
        fi
    done
}

mkdir -p "$system_tree" "$recovery_tree" "$OUTPUT_DIR"

echo "  → Creating the pristine MAN installation payload..."
cp -a "$TARGET_DIR/." "$system_tree/"
# ISO-only rebuilds reuse TARGET_DIR.  Refresh the small set of scripts that
# participate directly in first boot/installation so a fixed source checkout
# is never shadowed by an older staging tree.
for runtime_file in \
    etc/init.d/S35gio-modules \
    etc/login.defs \
    etc/network/interfaces \
    etc/init.d/S01splash \
    etc/init.d/S40network \
    etc/init.d/S41dhcpcd \
    etc/init.d/S42man-network \
    etc/init.d/S43man-networkd \
    etc/NetworkManager/NetworkManager.conf \
    etc/init.d/S46flatpak-flathub \
    etc/init.d/S99man-desktop \
    etc/init.d/S70seatd \
    usr/bin/man-cosmic-greeter-start \
    usr/bin/man-cosmic-greeter-ui \
    usr/bin/man-greeter-session \
    usr/bin/man-oobe-session \
    usr/bin/man-power-action \
    usr/bin/man-user-session \
    usr/sbin/man-oobe-apply \
    usr/sbin/man-powerd \
    usr/sbin/man-networkd \
    usr/sbin/man-install \
    usr/share/applications/com.man.Network.desktop; do
    if [ -f "$OVERLAY_DIR/$runtime_file" ]; then
        mkdir -p "$(dirname "$system_tree/$runtime_file")"
        cp "$OVERLAY_DIR/$runtime_file" "$system_tree/$runtime_file"
        chmod 0755 "$system_tree/$runtime_file"
    fi
done
if [ -f "$OVERLAY_DIR/etc/greetd/cosmic-greeter.toml" ]; then
    mkdir -p "$system_tree/etc/greetd"
    cp "$OVERLAY_DIR/etc/greetd/cosmic-greeter.toml" \
        "$system_tree/etc/greetd/cosmic-greeter.toml"
    chmod 0644 "$system_tree/etc/greetd/cosmic-greeter.toml"
fi
if [ -f "$OVERLAY_DIR/etc/NetworkManager/system-connections/MAN Wired.nmconnection" ]; then
    mkdir -p "$system_tree/etc/NetworkManager/system-connections"
    cp "$OVERLAY_DIR/etc/NetworkManager/system-connections/MAN Wired.nmconnection" \
        "$system_tree/etc/NetworkManager/system-connections/MAN Wired.nmconnection"
    chmod 0600 "$system_tree/etc/NetworkManager/system-connections/MAN Wired.nmconnection"
fi
if [ -f "$OVERLAY_DIR/etc/inittab" ]; then
    cp "$OVERLAY_DIR/etc/inittab" "$system_tree/etc/inittab"
    chmod 0644 "$system_tree/etc/inittab"
fi
# A standalone ISO rebuild can include a freshly cross-compiled Utilities
# binary without requiring a full Buildroot rebuild. This keeps the live
# installer and the installed payload in lockstep with UI fixes.
if [ -x "$UTILITIES_BINARY" ]; then
    cp "$UTILITIES_BINARY" "$system_tree/usr/bin/man-utilities"
    chmod 0755 "$system_tree/usr/bin/man-utilities"
fi
if [ -x "$SEATD_BINARY" ] && [ -f "$LIBSEAT_LIBRARY" ]; then
    cp "$SEATD_BINARY" "$system_tree/usr/bin/seatd"
    cp "$LIBSEAT_LIBRARY" "$system_tree/usr/lib/libseat.so.1"
    chmod 0755 "$system_tree/usr/bin/seatd"
fi
if [ -x "$AGREETY_BINARY" ]; then
    cp "$AGREETY_BINARY" "$system_tree/usr/bin/agreety"
    chmod 0755 "$system_tree/usr/bin/agreety"
fi
# Translation files are runtime data shared by the live installer and OOBE.
# Refresh the complete directory so ISO-only builds cannot retain partial or
# stale locale files from an older Buildroot target tree.
rm -rf "$system_tree/usr/share/man-utilities/i18n"
mkdir -p "$system_tree/usr/share/man-utilities"
cp -a "$OVERLAY_DIR/usr/share/man-utilities/i18n" \
      "$system_tree/usr/share/man-utilities/i18n"
# Boot assets and any older generated payloads are recovery media, not part of
# the installed normal root filesystem.
rm -rf "$system_tree/usr/share/man-installer/efi" \
       "$system_tree/usr/share/man-recovery" \
       "$system_tree/etc/man-recovery"
install_legal_docs "$system_tree"
(
    cd "$system_tree"
    find . -print0 | "$CPIO" -0 -o --format newc 2>/dev/null
) | "$GZIP" -9 >"$system_archive"

echo "  → Assembling the minimal MAN Recovery userspace..."
cp -a "$TARGET_DIR/." "$recovery_tree/"
# The live environment starts COSMIC itself, so it must receive the same
# startup wrappers as the installed payload. In particular S99 selects the
# seatd backend; retaining an older builtin-backend copy leaves the installer
# at its emergency root shell after seatd has claimed DRM/VT access.
for runtime_file in \
    etc/init.d/S35gio-modules \
    etc/init.d/S01splash \
    etc/init.d/S70seatd \
    etc/init.d/S99man-desktop \
    usr/bin/man-recovery-session; do
    if [ -f "$OVERLAY_DIR/$runtime_file" ]; then
        mkdir -p "$(dirname "$recovery_tree/$runtime_file")"
        cp "$OVERLAY_DIR/$runtime_file" "$recovery_tree/$runtime_file"
        chmod 0755 "$recovery_tree/$runtime_file"
    fi
done
if [ -x "$UTILITIES_BINARY" ]; then
    cp "$UTILITIES_BINARY" "$recovery_tree/usr/bin/man-utilities"
    chmod 0755 "$recovery_tree/usr/bin/man-utilities"
fi
if [ -x "$SEATD_BINARY" ] && [ -f "$LIBSEAT_LIBRARY" ]; then
    cp "$SEATD_BINARY" "$recovery_tree/usr/bin/seatd"
    cp "$LIBSEAT_LIBRARY" "$recovery_tree/usr/lib/libseat.so.1"
    chmod 0755 "$recovery_tree/usr/bin/seatd"
fi
# The live installer must never come from a stale Buildroot target tree during
# an ISO-only rebuild.  In particular this keeps its generated root=PARTUUID
# boot entry in sync with the source checkout.
if [ -f "$OVERLAY_DIR/usr/sbin/man-install" ]; then
    cp "$OVERLAY_DIR/usr/sbin/man-install" "$recovery_tree/usr/sbin/man-install"
    chmod 0755 "$recovery_tree/usr/sbin/man-install"
fi
rm -rf "$recovery_tree/usr/share/man-utilities/i18n"
mkdir -p "$recovery_tree/usr/share/man-utilities"
cp -a "$OVERLAY_DIR/usr/share/man-utilities/i18n" \
      "$recovery_tree/usr/share/man-utilities/i18n"
rm -rf "$recovery_tree/usr/share/man-installer/efi" \
       "$recovery_tree/usr/share/man-recovery" \
       "$recovery_tree/usr/src" \
       "$recovery_tree/usr/share/backgrounds" \
       "$recovery_tree/usr/share/doc" \
       "$recovery_tree/usr/share/man" \
       "$recovery_tree/usr/share/metainfo"
install_legal_docs "$recovery_tree"

# Recovery runs cosmic-comp directly. These normal desktop-shell components
# account for most of /usr/bin and would otherwise start a panel or dock.
for binary in \
    cosmic-app-library cosmic-applets cosmic-bg cosmic-files cosmic-files-applet \
    cosmic-idle cosmic-launcher cosmic-monitor cosmic-notifications cosmic-osd \
    cosmic-panel cosmic-randr cosmic-screenshot cosmic-session \
    cosmic-settings-daemon cosmic-workspaces cosmic-greeter cosmic-greeter-daemon; do
    rm -f "$recovery_tree/usr/bin/$binary"
done
find "$recovery_tree/usr/bin" -maxdepth 1 -type l -name 'cosmic-applet-*' -delete 2>/dev/null || true
rm -f "$recovery_tree/usr/bin/cosmic-app-list" \
      "$recovery_tree/usr/bin/cosmic-panel-button" \
      "$recovery_tree/usr/bin/man-oobe" \
      "$recovery_tree/usr/bin/man-oobe-autostart" \
      "$recovery_tree/usr/bin/app-depot" \
      "$recovery_tree/usr/bin/man-cosmic-greeter-start" \
      "$recovery_tree/usr/bin/man-greeter-session" \
      "$recovery_tree/usr/bin/man-user-session" \
      "$recovery_tree/usr/sbin/greetd" \
      "$recovery_tree/usr/sbin/man-oobe-apply" \
      "$recovery_tree/etc/xdg/autostart/com.man.OOBE.desktop" \
      "$recovery_tree/usr/share/applications/com.man.AppDepot.desktop" \
      "$recovery_tree/usr/share/metainfo/com.man.AppDepot.metainfo.xml"

mkdir -p "$recovery_tree/usr/share/man-recovery" \
         "$recovery_tree/usr/share/man-installer/efi/icons" \
         "$recovery_tree/usr/share/man-bootloader" \
         "$recovery_tree/usr/share/man-installer/efi/theme"
# The installer runs from this recovery image.  Always inject the source
# template here instead of inheriting a possibly stale copy from TARGET_DIR;
# otherwise an ISO rebuilt after a boot-config change would still install the
# old kernel command line on the destination disk.
cp "$BOOT_CONFIG_TEMPLATE" "$recovery_tree/usr/share/man-bootloader/man-boot.conf"
cp "$system_archive" "$recovery_tree/usr/share/man-recovery/man-system.cpio.gz"
cp "$BOOTLOADER_DIR/$EFI_BOOT" "$recovery_tree/usr/share/man-installer/efi/$EFI_BOOT"
cp "$KERNEL" "$recovery_tree/usr/share/man-installer/efi/$KERNEL_NAME"
for icon in os_man.png manrecovery.png transparent.png arrow_left.png arrow_right.png; do
    [ ! -f "$BOOTLOADER_DIR/icons/$icon" ] || \
        cp "$BOOTLOADER_DIR/icons/$icon" "$recovery_tree/usr/share/man-installer/efi/icons/"
done
find "$BOOTLOADER_DIR/theme" -maxdepth 1 -type f -name '*.png' -exec \
    cp {} "$recovery_tree/usr/share/man-installer/efi/theme/" \;
: >"$recovery_tree/etc/man-recovery"
printf '%s\n' 'MAN Recovery — installation and disk utilities' >"$recovery_tree/etc/issue"

pack_tree() {
    local tree="$1"
    local output="$2"
    rm -f "$output"
    (
        cd "$tree"
        find . -print0 | "$CPIO" -0 -o --format newc 2>/dev/null
    ) | "$GZIP" -9 >"$output"
}

# The installed image intentionally does not contain itself. If Recovery is
# used to reinstall its own disk, man-make-recovery recreates this file in RAM.
echo "  → Packing the installed MAN Recovery image..."
pack_tree "$recovery_tree" "$inner_image"

# Do not embed the Recovery image inside the live initramfs.  That would make
# the kernel unpack a second, nearly complete rootfs before the live system
# has even started, which can fail with "Initramfs unpacking failed: write
# error" on UEFI guests.  man-install regenerates this image under /run when
# it is absent, then copies that fresh image to the installed ESP.
rm -f "$recovery_tree/usr/share/man-installer/efi/man-recovery.cpio.gz"
echo "  → Packing the live MAN Utilities image..."
pack_tree "$recovery_tree" "$live_image"

inner_size="$(du -h "$inner_image" | awk '{print $1}')"
live_size="$(du -h "$live_image" | awk '{print $1}')"
echo "  ✓ MAN Recovery: $inner_image ($inner_size)"
echo "  ✓ Live Utilities: $live_image ($live_size)"
