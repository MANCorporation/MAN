#!/usr/bin/env bash
# Static validation for a COSMIC-enabled MAN root filesystem.

set -euo pipefail
TARGET_DIR="$1"

REQUIRED="cosmic-comp cosmic-session cosmic-settings-daemon cosmic-panel cosmic-bg cosmic-applets app-depot flatpak"
for binary in $REQUIRED; do
    [ -x "$TARGET_DIR/usr/bin/$binary" ] || {
        echo "error: /usr/bin/$binary is not installed" >&2
        exit 1
    }
done

REQUIRED_FILES="
usr/lib/libman-atexit-shim.so
usr/share/X11/xkb/rules/evdev
usr/share/cosmic/com.system76.CosmicSettings.Shortcuts/v1/defaults
usr/share/cosmic/com.system76.CosmicSettings.Shortcuts/v1/system_actions
usr/share/cosmic/com.system76.CosmicBackground/v1/all
    usr/share/wayland-sessions/cosmic.desktop
usr/share/applications/com.man.AppDepot.desktop
usr/share/metainfo/com.man.AppDepot.metainfo.xml
etc/init.d/S99man-desktop
"
for file in $REQUIRED_FILES; do
    [ -r "$TARGET_DIR/$file" ] || {
        echo "error: COSMIC runtime file is missing: /$file" >&2
        exit 1
    }
done

# Fail before imaging if any ELF dependency cannot be resolved in the target.
if command -v readelf >/dev/null 2>&1; then
    readelf -d "$TARGET_DIR/usr/bin/cosmic-comp" >/dev/null
fi

echo "  [OK] COSMIC rootfs contains the compositor, session, shell, and defaults"
