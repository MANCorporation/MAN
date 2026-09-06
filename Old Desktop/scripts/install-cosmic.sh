#!/usr/bin/env bash
# Install already-built COSMIC binaries and runtime data into a MAN rootfs.

set -euo pipefail

COSMIC_DIR="$1"
RUST_TRIPLE="$2"
TARGET_DIR="$3"
COSMIC_VERSION="$4"
XKB_DIR="$5"
GREETD_DIR="${6:-}"
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# GNU install treats -D as "create destination parents"; BSD/macOS install
# gives -D a different meaning. Normalize the GNU spelling used below so the
# same installer can populate a Linux staging tree from either host.
install() {
    case "${1:-}" in
        -Dm*)
            mode="${1#-Dm}"
            shift
            source="$1"
            destination="$2"
            mkdir -p "$(dirname "$destination")"
            command install -m "$mode" "$source" "$destination"
            ;;
        *) command install "$@" ;;
    esac
}

# Mesa's software renderer loads libstdc++ dynamically. The MAN glibc exports
# __cxa_atexit but not its legacy atexit wrapper, so install a session-local
# compatibility library rather than preloading it system-wide.
if [ -x "${COSMIC_CC:-}" ]; then
    "$COSMIC_CC" -shared -fPIC -Os \
        "$(dirname "$0")/../support/cosmic/atexit-shim.c" \
        -Wl,-soname,libman-atexit-shim.so \
        -o "$TARGET_DIR/usr/lib/libman-atexit-shim.so"
fi

install_tree() {
    source_dir="$1"
    destination_dir="$2"
    [ -d "$source_dir" ] || return 0
    find "$source_dir" -type f | while IFS= read -r source; do
        relative="${source#"$source_dir"/}"
        install -Dm0644 "$source" "$destination_dir/$relative"
    done
}

# The MAN artwork is deliberately installed in both its own theme and hicolor.
# COSMIC uses the freedesktop lookup rules for desktop entries, so the hicolor
# aliases make these raster icons available even before a user selects MAN as
# their preferred icon theme.
install_man_icons() {
    source_dir="$PROJECT_ROOT/ICONS"
    [ -d "$source_dir" ] || return 0

    find "$source_dir" -type f -name '*.png' | while IFS= read -r source; do
        relative="${source#"$source_dir"/}"
        # Flag icons live in a flags/ subdirectory and are installed separately
        # below with clean names (flag-XX) to keep the apps/ context tidy.
        case "$relative" in
            */flags/*) continue ;;
        esac
        size="${relative%%/*}"
        name="${relative#*/}"
        man_destination="$TARGET_DIR/usr/share/icons/MAN/$relative"
        cosmic_destination="$TARGET_DIR/usr/share/icons/Cosmic/$size/apps/$name"
        hicolor_destination="$TARGET_DIR/usr/share/icons/hicolor/$size/apps/$name"
        # BSD install's -D does not consistently make more than one missing
        # parent directory, unlike GNU install.
        mkdir -p "$(dirname "$man_destination")" "$(dirname "$cosmic_destination")" "$(dirname "$hicolor_destination")"
        install -m0644 "$source" "$man_destination"
        install -m0644 "$source" "$cosmic_destination"
        install -m0644 "$source" "$hicolor_destination"
    done

    # Install country-flag icons with stable, lookup-friendly names
    # (flag-XX) so the language chooser can resolve them via the standard
    # freedesktop icon lookup in any active theme.
    for size in 16x16 32x32 48x48 64x64; do
        flags_source_dir="$source_dir/$size/flags"
        [ -d "$flags_source_dir" ] || continue
        for flag_source in "$flags_source_dir"/*.png; do
            [ -f "$flag_source" ] || continue
            flag_basename="$(basename "$flag_source")"
            # Derive the two-letter ISO 3166-1 alpha-2 code from the file name
            # (e.g. "US United States.png" → "us").
            country_code="${flag_basename%% *}"
            flag_name="flag-$(printf '%s' "$country_code" | tr '[:upper:]' '[:lower:]')"
            for theme in MAN Cosmic hicolor; do
                dest_dir="$TARGET_DIR/usr/share/icons/$theme/$size/apps"
                mkdir -p "$dest_dir"
                install -m0644 "$flag_source" "$dest_dir/$flag_name.png"
            done
        done
    done

    mkdir -p "$TARGET_DIR/usr/share/icons/MAN"
    cat > "$TARGET_DIR/usr/share/icons/MAN/index.theme" <<'EOF'
[Icon Theme]
Name=MAN
Comment=MAN desktop artwork
Inherits=hicolor
Directories=16x16/apps,32x32/apps,48x48/apps,64x64/apps

[16x16/apps]
Size=16
Type=Fixed

[32x32/apps]
Size=32
Type=Fixed

[48x48/apps]
Size=48
Type=Fixed

[64x64/apps]
Size=64
Type=Fixed
EOF

    # Link application IDs used by COSMIC to the supplied MAN artwork.  The
    # links are created in each available raster size and remain valid in the
    # image because their target lives in the same theme directory.
    install_man_icon_alias() {
        source_name="$1"
        target_name="$2"
        for size_dir in "$TARGET_DIR"/usr/share/icons/MAN/*x*; do
            [ -f "$size_dir/$source_name.png" ] || continue
            size="$(basename "$size_dir")"
            ln -snf "$source_name.png" "$size_dir/$target_name.png"
            mkdir -p "$TARGET_DIR/usr/share/icons/Cosmic/$size/apps"
            ln -snf "$source_name.png" "$TARGET_DIR/usr/share/icons/Cosmic/$size/apps/$target_name.png"
            mkdir -p "$TARGET_DIR/usr/share/icons/hicolor/$size/apps"
            ln -snf "$source_name.png" "$TARGET_DIR/usr/share/icons/hicolor/$size/apps/$target_name.png"
        done
    }

    install_man_icon_alias App_Launchbox com.system76.CosmicAppLibrary
    install_man_icon_alias App_Launchbox com.system76.CosmicAppList
    install_man_icon_alias App_Tracker com.system76.CosmicFiles
    install_man_icon_alias App_Launchbox com.system76.CosmicLauncher
    install_man_icon_alias App_PowerStatus com.system76.CosmicMonitor
    install_man_icon_alias App_Screenshot com.system76.CosmicScreenshot
    install_man_icon_alias Prefs_Devices com.system76.CosmicSettings
    install_man_icon_alias App_Terminal com.system76.CosmicTerm
    install_man_icon_alias App_Workspaces com.system76.CosmicWorkspaces
    install_man_icon_alias App_Launchbox com.system76.CosmicPanelAppButton
    install_man_icon_alias App_Launchbox com.system76.CosmicPanelLauncherButton
    install_man_icon_alias App_Workspaces com.system76.CosmicPanelWorkspacesButton
    install_man_icon_alias Prefs_Sounds com.system76.CosmicAppletAudio-symbolic
    install_man_icon_alias Battery_Low com.system76.CosmicAppletBattery-symbolic
    install_man_icon_alias Prefs_Bluetooth com.system76.CosmicAppletBluetooth-symbolic
    install_man_icon_alias Prefs_Keyboard com.system76.CosmicAppletInputSources-symbolic
    install_man_icon_alias Tray_Network com.system76.CosmicAppletNetwork-symbolic
    install_man_icon_alias App_Mail com.system76.CosmicAppletNotifications-symbolic
    install_man_icon_alias App_PowerStatus com.system76.CosmicAppletPower-symbolic
    install_man_icon_alias App_Workspaces com.system76.CosmicAppletTiling-symbolic
    install_man_icon_alias App_Clock com.system76.CosmicAppletTime-symbolic
    install_man_icon_alias Deskbar_Window_Hidden com.system76.CosmicAppletMinimize
    install_man_icon_alias App_PowerStatus com.system76.CosmicAppletStatusArea
    install_man_icon_alias App_Workspaces com.system76.CosmicAppletWorkspaces
    install_man_icon_alias App_PackageInstaller com.man.Install
    install_man_icon_alias App_HaikuDepot com.man.AppDepot
    install_man_icon_alias Misc_Book com.man.Help
    install_man_icon_alias Prefs_DriveSetup com.man.Diskulator
    install_man_icon_alias App_About com.man.Utilities
    install_man_icon_alias App_About com.man.OOBE
    install_man_icon_alias Action_Login com.system76.CosmicGreeter

    # Freedesktop names used by COSMIC settings and utility views.  MAN's icon
    # collection predates those names, so supply stable aliases at every size.
    install_man_icon_alias Prefs_Backgrounds preferences-appearance
    install_man_icon_alias Prefs_Backgrounds preferences-desktop-wallpaper
    install_man_icon_alias Prefs_Devices preferences-input-devices
    install_man_icon_alias Prefs_Keyboard preferences-desktop-keyboard
    install_man_icon_alias Prefs_Touchpad2 preferences-touchpad
    install_man_icon_alias Prefs_Mouse preferences-mouse
    install_man_icon_alias Prefs_Network preferences-network-and-wireless
    install_man_icon_alias Prefs_Bluetooth preferences-bluetooth
    install_man_icon_alias Prefs_Sound preferences-sound
    install_man_icon_alias Device_Harddisk drive-harddisk
    install_man_icon_alias Device_Harddisk drive-harddisk-symbolic
    install_man_icon_alias App_People system-users
    install_man_icon_alias Prefs_Locale preferences-desktop-locale
    install_man_icon_alias Prefs_Appearance preferences-desktop-appearance
    install_man_icon_alias Prefs_Appearance preferences-appearance
    install_man_icon_alias Prefs_Fonts preferences-desktop-font
    install_man_icon_alias Prefs_Touchpad2 preferences-touchpad
    install_man_icon_alias App_SoftwareUpdater preferences-system-updates
    install_man_icon_alias App_ThemeManager applications-graphics
    install_man_icon_alias App_Workspaces view-grid-symbolic
    install_man_icon_alias App_Tracker view-list-symbolic
    install_man_icon_alias App_Magnify preferences-desktop-accessibility
    install_man_icon_alias App_PowerStatus preferences-system-time
    install_man_icon_alias Prefs_Locale mark-location-symbolic
    install_man_icon_alias App_Launchbox applications-other
    install_man_icon_alias App_GnuPG preferences-system-privacy
    install_man_icon_alias Prefs_Screen preferences-desktop
    install_man_icon_alias App_bootman system-run-symbolic
    install_man_icon_alias Action_Login emblem-ok-symbolic
    install_man_icon_alias App_PowerStatus utilities-system-monitor
    install_man_icon_alias Action_GoDown_3 go-bottom-symbolic
    install_man_icon_alias Prefs_Screen view-fullscreen-symbolic
    install_man_icon_alias App_Tracker view-list-symbolic
}

# Install the MAN boot config template so man-install can generate refind.conf
# on the installed system's ESP.
BOOTCONF_SOURCE="${PROJECT_ROOT}/scripts/bootloader/man-boot.conf"
BOOTCONF_DEST="$TARGET_DIR/usr/share/man-bootloader/man-boot.conf"
if [ -f "$BOOTCONF_SOURCE" ]; then
    mkdir -p "$(dirname "$BOOTCONF_DEST")"
    install -m0644 "$BOOTCONF_SOURCE" "$BOOTCONF_DEST"
    echo "  → Installed MAN boot config template → /usr/share/man-bootloader/man-boot.conf"
fi

for xkb_section in compat geometry keycodes rules symbols types; do
    install_tree "$XKB_DIR/$xkb_section" "$TARGET_DIR/usr/share/X11/xkb/$xkb_section"
done

install -Dm0755 "$PROJECT_ROOT/support/man-splash/target/$RUST_TRIPLE/release/man-splash" \
    "$TARGET_DIR/usr/bin/man-splash"
install -Dm0755 "$COSMIC_DIR/target/$RUST_TRIPLE/release/man-utilities" \
    "$TARGET_DIR/usr/bin/man-utilities"
if [ -x "$COSMIC_DIR/target/$RUST_TRIPLE/release/cosmic-store" ]; then
    install -Dm0755 "$COSMIC_DIR/target/$RUST_TRIPLE/release/cosmic-store" \
        "$TARGET_DIR/usr/bin/app-depot"
else
    echo "warning: cosmic-store not built (MAN_UTILITIES_ONLY=1); skipping app-depot install" >&2
fi
install -Dm0644 "$PROJECT_ROOT/overlay/usr/share/applications/com.man.AppDepot.desktop" \
    "$TARGET_DIR/usr/share/applications/com.man.AppDepot.desktop"
install -Dm0644 "$PROJECT_ROOT/overlay/usr/share/metainfo/com.man.AppDepot.metainfo.xml" \
    "$TARGET_DIR/usr/share/metainfo/com.man.AppDepot.metainfo.xml"
# These scripts are part of App Depot's runtime contract: they establish the
# Flathub user remote before COSMIC starts, so the freshly installed store can
# load its AppStream catalog on first launch.
install -Dm0755 "$PROJECT_ROOT/overlay/usr/bin/man-flatpak-setup" \
    "$TARGET_DIR/usr/bin/man-flatpak-setup"
install -Dm0755 "$PROJECT_ROOT/overlay/usr/bin/tar" \
    "$TARGET_DIR/usr/bin/tar"
install -Dm0755 "$PROJECT_ROOT/overlay/usr/bin/man-user-session" \
    "$TARGET_DIR/usr/bin/man-user-session"
install -Dm0755 "$PROJECT_ROOT/overlay/usr/bin/man-help" \
    "$TARGET_DIR/usr/bin/man-help"
install -Dm0644 "$PROJECT_ROOT/overlay/etc/profile.d/cosmic-simple.sh" \
    "$TARGET_DIR/etc/profile.d/cosmic-simple.sh"
install -Dm0644 "$PROJECT_ROOT/overlay/usr/share/applications/com.man.Help.desktop" \
    "$TARGET_DIR/usr/share/applications/com.man.Help.desktop"
for desktop_file in com.man.Diskulator.desktop com.man.Network.desktop com.man.mupdf.desktop com.man.mpv.desktop; do
    install -Dm0644 "$PROJECT_ROOT/overlay/usr/share/applications/$desktop_file" \
        "$TARGET_DIR/usr/share/applications/$desktop_file"
done
install -Dm0644 "$PROJECT_ROOT/overlay/usr/share/man/help/MAN-HELP.txt" \
    "$TARGET_DIR/usr/share/man/help/MAN-HELP.txt"
# Give every MAN-supported locale a deterministic local article path.  The
# article falls back to the canonical document until a curated translation is
# available, so Help never disappears when the user changes language.
for locale_file in "$PROJECT_ROOT"/overlay/usr/share/man-utilities/i18n/*.json; do
    locale_name="${locale_file##*/}"
    locale_name="${locale_name%.json}"
    install -Dm0644 "$PROJECT_ROOT/overlay/usr/share/man/help/MAN-HELP.txt" \
        "$TARGET_DIR/usr/share/man/help/$locale_name/MAN-HELP.txt"
done
install -Dm0755 "$PROJECT_ROOT/overlay/etc/init.d/S35gio-modules" \
    "$TARGET_DIR/etc/init.d/S35gio-modules"
install -Dm0755 "$PROJECT_ROOT/overlay/etc/init.d/S40network" \
    "$TARGET_DIR/etc/init.d/S40network"
install -Dm0755 "$PROJECT_ROOT/overlay/etc/init.d/S41dhcpcd" \
    "$TARGET_DIR/etc/init.d/S41dhcpcd"
install -Dm0644 "$PROJECT_ROOT/overlay/etc/NetworkManager/NetworkManager.conf" \
    "$TARGET_DIR/etc/NetworkManager/NetworkManager.conf"
install -Dm0755 "$PROJECT_ROOT/overlay/etc/NetworkManager/dispatcher.d/20-man-resolver" \
    "$TARGET_DIR/etc/NetworkManager/dispatcher.d/20-man-resolver"
install -Dm0755 "$PROJECT_ROOT/overlay/etc/init.d/S47man-network-recovery" \
    "$TARGET_DIR/etc/init.d/S47man-network-recovery"
install -Dm0755 "$PROJECT_ROOT/overlay/etc/init.d/S46flatpak-flathub" \
    "$TARGET_DIR/etc/init.d/S46flatpak-flathub"
install -Dm0600 "$PROJECT_ROOT/overlay/etc/NetworkManager/system-connections/MAN Wired.nmconnection" \
    "$TARGET_DIR/etc/NetworkManager/system-connections/MAN Wired.nmconnection"
ln -snf man-utilities "$TARGET_DIR/usr/bin/man-diskulator"
ln -snf man-utilities "$TARGET_DIR/usr/bin/man-oobe"
ln -snf man-utilities "$TARGET_DIR/usr/bin/man-network"
install -Dm0755 "$COSMIC_DIR/target/$RUST_TRIPLE/release/man-guide" \
    "$TARGET_DIR/usr/bin/man-guide"
if [ -x "${COSMIC_STRIP:-}" ]; then
    "$COSMIC_STRIP" --strip-unneeded \
        "$TARGET_DIR/usr/bin/man-splash" \
        "$TARGET_DIR/usr/bin/man-utilities"
    [ -x "$TARGET_DIR/usr/bin/app-depot" ] && "$COSMIC_STRIP" --strip-unneeded "$TARGET_DIR/usr/bin/app-depot" || true
fi

install -Dm0755 "$PROJECT_ROOT/overlay/usr/sbin/man-update" \
    "$TARGET_DIR/usr/sbin/man-update"
install -Dm0755 "$PROJECT_ROOT/overlay/usr/sbin/man-install" \
    "$TARGET_DIR/usr/sbin/man-install" 2>/dev/null || true
install -Dm0755 "$PROJECT_ROOT/overlay/usr/sbin/man-diskutil" \
    "$TARGET_DIR/usr/sbin/man-diskutil" 2>/dev/null || true
mkdir -p "$TARGET_DIR/var/lib/man"
install -m0644 "$PROJECT_ROOT/overlay/var/lib/man/man-beta-updates" \
    "$TARGET_DIR/var/lib/man/man-beta-updates" 2>/dev/null || true

if [ "${MAN_UTILITIES_ONLY:-0}" = 1 ]; then
    echo "  → Installed MAN Utilities-only update"
    exit 0
fi

# The release repository stores the evdev rules as ordered source fragments;
# its Meson build normally merges them. Generate the same runtime file without
# pulling X.Org host tools into this Wayland-only image.
XKB_RULE_PARTS="
0000-hdr.part 0001-lists.part 0002-evdev.lists.part
0004-evdev.m_k.part 0005-l1_k.part 0006-l_k.part 0007-o_k.part
0008-ml_g.part 0009-m_g.part 0011-mlv_s.part 0013-ml_s.part
0015-ml1_s.part 0018-ml2_s.part 0020-ml3_s.part 0022-ml4_s.part
0026-evdev.m_s.part 0027-evdev.ml_s1.part 0033-ml_c.part
0034-ml1_c.part 0035-m_t.part 0036-lo_s.part 0037-l1o_s.part
0038-l2o_s.part 0039-l3o_s.part 0040-l4o_s.part 0042-o_s.part
0043-o_c.part 0044-o_t.part
"
# shellcheck disable=SC2086 # ordered filenames are intentional argv entries.
python3 "$XKB_DIR/rules/merge.py" --srcdir "$XKB_DIR/rules" $XKB_RULE_PARTS \
    > "$TARGET_DIR/usr/share/X11/xkb/rules/evdev"

install_xdgen() {
    source_dir="$1"
    [ -d "$source_dir" ] || return 0
    find "$source_dir" -maxdepth 1 -type f -name '*.desktop' | while IFS= read -r source; do
        install -Dm0644 "$source" "$TARGET_DIR/usr/share/applications/$(basename "$source")"
    done
    find "$source_dir" -maxdepth 1 -type f -name '*.metainfo.xml' | while IFS= read -r source; do
        install -Dm0644 "$source" "$TARGET_DIR/usr/share/metainfo/$(basename "$source")"
    done
}

install_binary() {
    name="$1"
    source="$(find "$COSMIC_DIR" -path "*/target/$RUST_TRIPLE/release/$name" -type f -perm -111 -print -quit)"
    [ -x "$source" ] || { echo "error: required COSMIC binary is missing: $source" >&2; exit 1; }
    install -Dm0755 "$source" "$TARGET_DIR/usr/bin/$name"
    if [ -x "${COSMIC_STRIP:-}" ]; then
        "$COSMIC_STRIP" --strip-unneeded "$TARGET_DIR/usr/bin/$name"
    fi
}

REQUIRED_BINARIES="cosmic-comp cosmic-session cosmic-settings-daemon cosmic-notifications cosmic-panel cosmic-applets cosmic-app-library cosmic-launcher cosmic-workspaces cosmic-osd cosmic-bg cosmic-idle"
for binary in $REQUIRED_BINARIES; do
    install_binary "$binary"
done

install_binary cosmic-greeter
install_binary cosmic-greeter-daemon
[ -n "$GREETD_DIR" ] && [ -x "$GREETD_DIR/target/$RUST_TRIPLE/release/greetd" ] || {
    echo "error: greetd was not built for $RUST_TRIPLE" >&2
    exit 1
}
install -Dm0755 "$GREETD_DIR/target/$RUST_TRIPLE/release/greetd" "$TARGET_DIR/usr/sbin/greetd"
install -Dm0644 "$COSMIC_DIR/cosmic-greeter/dbus/com.system76.CosmicGreeter.conf" \
    "$TARGET_DIR/usr/share/dbus-1/system.d/com.system76.CosmicGreeter.conf"

install_binary cosmic-term

OPTIONAL_BINARIES="cosmic-settings cosmic-files cosmic-files-applet cosmic-monitor cosmic-randr cosmic-screenshot"
for binary in $OPTIONAL_BINARIES; do
    source="$(find "$COSMIC_DIR" -path "*/target/$RUST_TRIPLE/release/$binary" -type f -perm -111 -print -quit)"
    if [ -n "$source" ]; then
        install -Dm0755 "$source" "$TARGET_DIR/usr/bin/$binary"
        if [ -x "${COSMIC_STRIP:-}" ]; then
            "$COSMIC_STRIP" --strip-unneeded "$TARGET_DIR/usr/bin/$binary"
        fi
    fi
done

# Applet desktop entries invoke these names. COSMIC ships one multicall
# executable, so the individual applets are links to cosmic-applets.
APPLET_NAMES="cosmic-panel-button cosmic-app-list cosmic-applet-a11y cosmic-applet-audio cosmic-applet-input-sources cosmic-applet-battery cosmic-applet-bluetooth cosmic-applet-minimize cosmic-applet-network cosmic-applet-notifications cosmic-applet-power cosmic-applet-status-area cosmic-applet-tiling cosmic-applet-time cosmic-applet-workspaces"
for applet in $APPLET_NAMES; do
    ln -snf cosmic-applets "$TARGET_DIR/usr/bin/$applet"
done

install -Dm0644 "$COSMIC_DIR/cosmic-comp/data/keybindings.ron" \
    "$TARGET_DIR/usr/share/cosmic/com.system76.CosmicSettings.Shortcuts/v1/defaults"
install -Dm0644 "$COSMIC_DIR/cosmic-comp/data/tiling-exceptions.ron" \
    "$TARGET_DIR/usr/share/cosmic/com.system76.CosmicSettings.WindowRules/v1/tiling_exception_defaults"
install -Dm0644 "$COSMIC_DIR/cosmic-session/data/cosmic-mimeapps.list" \
    "$TARGET_DIR/usr/share/applications/cosmic-mimeapps.list"
install -Dm0644 "$COSMIC_DIR/cosmic-session/data/cosmic.desktop" \
    "$TARGET_DIR/usr/share/wayland-sessions/cosmic.desktop"
sed -i.bak 's#^Exec=.*#Exec=/usr/bin/man-user-session#' \
    "$TARGET_DIR/usr/share/wayland-sessions/cosmic.desktop"
rm -f "$TARGET_DIR/usr/share/wayland-sessions/cosmic.desktop.bak"

install_tree "$COSMIC_DIR/cosmic-panel/data/default_schema" "$TARGET_DIR/usr/share/cosmic"
install_tree "$COSMIC_DIR/cosmic-bg/data/v1" \
    "$TARGET_DIR/usr/share/cosmic/com.system76.CosmicBackground/v1"

# Install generated applet launchers and all applet icons/default schemas.
install_tree "$COSMIC_DIR/cosmic-applets/target/xdgen" "$TARGET_DIR/usr/share/applications"
for data_dir in "$COSMIC_DIR/cosmic-applets"/*/data; do
    [ -d "$data_dir/icons" ] && install_tree "$data_dir/icons" "$TARGET_DIR/usr/share/icons/hicolor"
    [ -d "$data_dir/default_schema" ] && install_tree "$data_dir/default_schema" "$TARGET_DIR/usr/share/cosmic"
done

# Shell components are started directly by cosmic-session, while these entries
# provide their metadata and icons to the launcher and panel.
for component in cosmic-applibrary cosmic-launcher; do
    install_xdgen "$COSMIC_DIR/$component/data"
    install_tree "$COSMIC_DIR/$component/data/icons" "$TARGET_DIR/usr/share/icons/hicolor/scalable/apps"
done
install -Dm0644 "$COSMIC_DIR/cosmic-workspaces-epoch/data/com.system76.CosmicWorkspaces.desktop" \
    "$TARGET_DIR/usr/share/applications/com.system76.CosmicWorkspaces.desktop"
install -Dm0644 "$COSMIC_DIR/cosmic-workspaces-epoch/data/com.system76.CosmicWorkspaces.svg" \
    "$TARGET_DIR/usr/share/icons/hicolor/scalable/apps/com.system76.CosmicWorkspaces.svg"

# Install application data only when its corresponding optional binary exists.
for component in cosmic-settings cosmic-files cosmic-term cosmic-monitor; do
    binary="${component}"
    [ -x "$TARGET_DIR/usr/bin/$binary" ] || continue
    install_xdgen "$COSMIC_DIR/$component/target/xdgen"
done
if [ -x "$TARGET_DIR/usr/bin/cosmic-settings" ]; then
    install_tree "$COSMIC_DIR/cosmic-settings/resources/default_schema" "$TARGET_DIR/usr/share/cosmic"
    install_tree "$COSMIC_DIR/cosmic-settings/resources/icons" "$TARGET_DIR/usr/share/icons/hicolor"
    [ -f "$COSMIC_DIR/cosmic-settings/resources/com.system76.CosmicSettings.metainfo.xml" ] && \
        install -Dm0644 "$COSMIC_DIR/cosmic-settings/resources/com.system76.CosmicSettings.metainfo.xml" \
        "$TARGET_DIR/usr/share/metainfo/com.system76.CosmicSettings.metainfo.xml"
fi
for component in cosmic-files cosmic-term cosmic-monitor; do
    [ -x "$TARGET_DIR/usr/bin/$component" ] && \
        install_tree "$COSMIC_DIR/$component/res/icons/hicolor" "$TARGET_DIR/usr/share/icons/hicolor"
done
if [ -x "$TARGET_DIR/usr/bin/cosmic-screenshot" ]; then
    install -Dm0644 "$COSMIC_DIR/cosmic-screenshot/resources/com.system76.CosmicScreenshot.desktop" \
        "$TARGET_DIR/usr/share/applications/com.system76.CosmicScreenshot.desktop"
    install_tree "$COSMIC_DIR/cosmic-screenshot/resources/icons/hicolor" "$TARGET_DIR/usr/share/icons/hicolor"
fi

install -Dm0644 "$COSMIC_DIR/cosmic-settings-daemon/data/system_actions.ron" \
    "$TARGET_DIR/usr/share/cosmic/com.system76.CosmicSettings.Shortcuts/v1/system_actions"
install -Dm0644 "$COSMIC_DIR/cosmic-settings-daemon/data/polkit-1/rules.d/cosmic-settings-daemon.rules" \
    "$TARGET_DIR/usr/share/polkit-1/rules.d/cosmic-settings-daemon.rules"

# These projects contain architecture-independent runtime assets.
install_tree "$COSMIC_DIR/cosmic-icons/freedesktop" "$TARGET_DIR/usr/share/icons/Cosmic"
install_tree "$COSMIC_DIR/cosmic-icons/extra" "$TARGET_DIR/usr/share/icons/Cosmic"
[ -f "$COSMIC_DIR/cosmic-icons/index.theme" ] && install -Dm0644 \
    "$COSMIC_DIR/cosmic-icons/index.theme" "$TARGET_DIR/usr/share/icons/Cosmic/index.theme"
install_man_icons
# Only wallpaper media are runtime assets; exclude repository metadata and
# packaging sources from the embedded image.
if [ -d "$COSMIC_DIR/cosmic-wallpapers/wallpapers" ]; then
    install_tree "$COSMIC_DIR/cosmic-wallpapers/wallpapers" "$TARGET_DIR/usr/share/backgrounds/cosmic"
else
    find "$COSMIC_DIR/cosmic-wallpapers" -type f \
        \( -name '*.jpg' -o -name '*.jpeg' -o -name '*.png' -o -name '*.webp' -o -name '*.jxl' \) | \
    while IFS= read -r source; do
        install -Dm0644 "$source" "$TARGET_DIR/usr/share/backgrounds/cosmic/$(basename "$source")"
    done
fi

mkdir -p "$TARGET_DIR/usr/share/man-desktop"
printf '%s\n' "$COSMIC_VERSION" > "$TARGET_DIR/usr/share/man-desktop/version"

# The COSMIC payload includes GPL- and MPL-covered software. Preserve the
# applicable license texts and attributions in the installed rootfs; never
# present the entire distribution as proprietary.
LEGAL_DOC_DIR="$TARGET_DIR/usr/share/doc/MAN"
mkdir -p "$LEGAL_DOC_DIR"
install -Dm0644 "$PROJECT_ROOT/LICENSE" "$LEGAL_DOC_DIR/LICENSE"
install -Dm0644 "$PROJECT_ROOT/LICENSES/LicenseRef-MAN-Proprietary.txt" \
    "$LEGAL_DOC_DIR/MAN-PROPRIETARY-LICENSE.txt"
install -Dm0644 "$PROJECT_ROOT/THIRD_PARTY_NOTICES.md" \
    "$LEGAL_DOC_DIR/THIRD_PARTY_NOTICES.md"
install -Dm0644 "$PROJECT_ROOT/DISTRIBUTION_COMPLIANCE.md" \
    "$LEGAL_DOC_DIR/DISTRIBUTION_COMPLIANCE.md"
install -Dm0644 "$PROJECT_ROOT/ASSET_PROVENANCE.md" \
    "$LEGAL_DOC_DIR/ASSET_PROVENANCE.md"
install -Dm0644 "$PROJECT_ROOT/scripts/bootloader/THIRD_PARTY-ICON-LICENSES.md" \
    "$LEGAL_DOC_DIR/rEFInd-ICON-NOTICES.md"
install -Dm0644 "$PROJECT_ROOT/support/pam-client/LICENSE" \
    "$LEGAL_DOC_DIR/MPL-2.0-pam-client.txt"
install -Dm0644 "$PROJECT_ROOT/toolchain/COPYING" \
    "$LEGAL_DOC_DIR/GPL-2.0-or-later.txt"
install -Dm0644 "$PROJECT_ROOT/LICENSES/MIT.txt" \
    "$LEGAL_DOC_DIR/Haiku-ICONS-MIT.txt"
for gpl3_source in "$COSMIC_DIR/cosmic-comp/LICENSE" "$COSMIC_DIR/cosmic-greeter/LICENSE"; do
    if [ -f "$gpl3_source" ]; then
        install -Dm0644 "$gpl3_source" "$LEGAL_DOC_DIR/GPL-3.0.txt"
        break
    fi
done

# Install the MAN boot theme (background + entry icons) into the EFI
# payload directory so the runtime installer can copy it to the ESP.
THEME_SOURCE="${PROJECT_ROOT}/scripts/bootloader/theme"
THEME_DEST="$TARGET_DIR/usr/share/man-installer/efi/theme"
if [ -d "$THEME_SOURCE" ]; then
    mkdir -p "$THEME_DEST"
    for png in "$THEME_SOURCE"/*.png; do
        [ -f "$png" ] && install -m0644 "$png" "$TARGET_DIR/usr/share/man-installer/efi/theme/$(basename "$png")"
    done
    echo "  → Installed MAN boot theme → /usr/share/man-installer/efi/theme/"
fi

"$(dirname "$0")/verify-cosmic-rootfs.sh" "$TARGET_DIR"
