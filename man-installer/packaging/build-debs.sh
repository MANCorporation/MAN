#!/bin/bash
# Build man-system-tools and man-installer .deb packages.
#   packaging/build-debs.sh [output-dir]
# Needs a Rust toolchain and pkg-config files for GTK 4 >= 4.16 and
# libadwaita >= 1.6 (set PKG_CONFIG_PATH / PKG_CONFIG_SYSROOT_DIR if they are
# not installed system-wide).
set -euo pipefail
here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
out=${1:-$here/target/debs}
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$here/Cargo.toml" | head -n1)
data=$here/data

cargo build --release --manifest-path "$here/Cargo.toml" --workspace
bin=$here/target/release
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
mkdir -p "$out"

put() { # put SRC DEST MODE
    install -D -m "$3" "$1" "$2"
}
# /usr/share/doc/<package>/copyright: MAN's license + the Rust libraries in it.
copyright() { # copyright PKGROOT
    local d="$1/usr/share/doc/$(basename "$1")"
    mkdir -p "$d"
    { cat "$here/../LICENSE"; echo; echo; cat "$here/THIRD-PARTY-RUST.txt"; } > "$d/copyright"
    chmod 0644 "$d/copyright"
}

# ── man-system-tools ────────────────────────────────────────────────
r=$stage/man-system-tools
put "$bin/man-installer-backend" "$r/usr/libexec/man-installer/man-installer-backend" 0755
put "$bin/man-snapshots" "$r/usr/bin/man-snapshots" 0755
put "$data/applications/com.man.Snapshots.desktop" "$r/usr/share/applications/com.man.Snapshots.desktop" 0644
put "$bin/man-oobe" "$r/usr/bin/man-oobe" 0755
put "$bin/man-update" "$r/usr/bin/man-update" 0755
put "$bin/man-update-check" "$r/usr/bin/man-update-check" 0755
put "$data/updates/man-update-check.service" "$r/usr/lib/systemd/user/man-update-check.service" 0644
put "$data/updates/man-update-check.timer" "$r/usr/lib/systemd/user/man-update-check.timer" 0644
put "$data/updates/com.man.UpdateCheck.service" "$r/usr/share/dbus-1/services/com.man.UpdateCheck.service" 0644
put "$data/updates/com.man.UpdateCheck.desktop" "$r/usr/share/applications/com.man.UpdateCheck.desktop" 0644
put "$data/schemas/com.man.Updates.gschema.xml" "$r/usr/share/glib-2.0/schemas/com.man.Updates.gschema.xml" 0644
put "$data/flatpak/man-flatpak-sandbox" "$r/usr/libexec/man/man-flatpak-sandbox" 0755
put "$data/polkit/com.man.flatpak.policy" "$r/usr/share/polkit-1/actions/com.man.flatpak.policy" 0644
put "$bin/man-mkupd" "$r/usr/bin/man-mkupd" 0755
put "$data/version/man-set-version" "$r/usr/bin/man-set-version" 0755
put "$bin/man-app-installer" "$r/usr/bin/man-app-installer" 0755
put "$bin/man-app-installer-helper" "$r/usr/libexec/man-installer/man-app-installer-helper" 0755
put "$data/applications/com.man.AppInstaller.desktop" "$r/usr/share/applications/com.man.AppInstaller.desktop" 0644
put "$data/applications/gnome-mimeapps.list" "$r/etc/xdg/gnome-mimeapps.list" 0644
put "$data/icons/com.man.AppInstaller.svg" "$r/usr/share/icons/hicolor/scalable/apps/com.man.AppInstaller.svg" 0644
put "$data/polkit/com.man.apps.policy" "$r/usr/share/polkit-1/actions/com.man.apps.policy" 0644
put "$data/applications/com.man.Updater.desktop" "$r/usr/share/applications/com.man.Updater.desktop" 0644
put "$data/autostart/com.man.UpdateNotice.desktop" "$r/etc/xdg/autostart/com.man.UpdateNotice.desktop" 0644
put "$data/icons/com.man.Updater.svg" "$r/usr/share/icons/hicolor/scalable/apps/com.man.Updater.svg" 0644
put "$data/icons/com.man.Updates-symbolic.svg" "$r/usr/share/icons/hicolor/symbolic/apps/com.man.Updates-symbolic.svg" 0644
put "$data/mime/man-update.xml" "$r/usr/share/mime/packages/man-update.xml" 0644
put "$data/keys/man.pub" "$r/usr/share/man/update-keys/man.pub" 0644
put "$data/grub/41_man_update" "$r/etc/grub.d/41_man_update" 0755
put "$data/initramfs/man-update.premount" "$r/usr/share/initramfs-tools/scripts/local-premount/man-update" 0755
put "$data/oobe/man-oobe-session" "$r/usr/libexec/man-oobe/man-oobe-session" 0755
put "$data/oobe/man-oobe.desktop" "$r/usr/share/wayland-sessions/man-oobe.desktop" 0644
put "$data/oobe/com.man.Oobe.desktop" "$r/usr/share/applications/com.man.Oobe.desktop" 0644
put "$data/icons/com.man.Oobe.svg" "$r/usr/share/icons/hicolor/scalable/apps/com.man.Oobe.svg" 0644
put "$data/icons/com.man.Globe.svg" "$r/usr/share/icons/hicolor/scalable/apps/com.man.Globe.svg" 0644
put "$data/oobe/49-man-oobe.rules" "$r/usr/share/polkit-1/rules.d/49-man-oobe.rules" 0644
put "$data/initramfs/man-encrypt.hook" "$r/usr/share/initramfs-tools/hooks/man-encrypt" 0755
put "$data/initramfs/man-encrypt.premount" "$r/usr/share/initramfs-tools/scripts/init-premount/man-encrypt" 0755
put "$data/metainfo/com.man.Snapshots.metainfo.xml" "$r/usr/share/metainfo/com.man.Snapshots.metainfo.xml" 0644
put "$data/icons/com.man.Snapshots.svg" "$r/usr/share/icons/hicolor/scalable/apps/com.man.Snapshots.svg" 0644
put "$data/polkit/com.man.system.policy" "$r/usr/share/polkit-1/actions/com.man.system.policy" 0644
put "$data/grub/42_man_snapshots" "$r/etc/grub.d/42_man_snapshots" 0755
put "$data/grub/43_man_windows" "$r/etc/grub.d/43_man_windows" 0755
put "$data/duhach/man-duhach-cleanup" "$r/usr/libexec/man/man-duhach-cleanup" 0755
put "$data/initramfs/man-rollback.hook" "$r/usr/share/initramfs-tools/hooks/man-rollback" 0755
put "$data/initramfs/man-rollback.premount" "$r/usr/share/initramfs-tools/scripts/local-premount/man-rollback" 0755
for u in man-snapshot-daily.service man-snapshot-daily.timer man-boot-cleanup.service man-boot-cleanup.timer man-oobe-cleanup.service man-encrypt.service man-update-finish.service man-duhach-cleanup.service; do
    put "$data/systemd/$u" "$r/usr/lib/systemd/system/$u" 0644
done
put "$data/apt/80man-snapshot" "$r/etc/apt/apt.conf.d/80man-snapshot" 0644
put "$data/apt/man-debian.sources" "$r/etc/apt/sources.list.d/man-debian.sources" 0644
put "$data/apt/man-debian.pref" "$r/etc/apt/preferences.d/man-debian" 0644
put "$data/config/snapshots.conf" "$r/etc/man/snapshots.conf" 0644
for l in man-logo man-logo-text man-logo-text-dark; do
    put "$data/logo/$l.svg" "$r/usr/share/icons/hicolor/scalable/apps/$l.svg" 0644
done
put "$data/gdm/95-man-logo" "$r/usr/share/gdm/dconf/95-man-logo" 0644
put "$data/gdm/man-gdm-logo.png" "$r/usr/share/pixmaps/man-gdm-logo.png" 0644
put "$data/schemas/20_man-desktop.gschema.override" "$r/usr/share/glib-2.0/schemas/20_man-desktop.gschema.override" 0644
copyright "$r"
mkdir -p "$r/DEBIAN"
cat > "$r/DEBIAN/control" <<EOF
Package: man-system-tools
Version: $version
Architecture: amd64
Maintainer: MAN Developers <dev@man.local>
Section: admin
Priority: optional
Depends: libc6 (>= 2.34), libgtk-4-1 (>= 4.16), libadwaita-1-0 (>= 1.6), btrfs-progs, rsync, gdisk, parted, dosfstools, e2fsprogs, cryptsetup, cryptsetup-initramfs, initramfs-tools, busybox, grub-efi-amd64-bin, grub2-common, pkexec, polkitd, util-linux, mutter, network-manager, dbus-daemon, dconf-cli, zstd, tar, efibootmgr, locales, iso-codes, tzdata, curl, apt, flatpak, libfuse2t64
Description: MAN system backend, MAN Setup and snapshot manager
 Privileged backend used by the MAN installer, the Snapshots app for
 creating and restoring Btrfs system snapshots, MAN Setup (first-start
 setup with optional disk encryption), daily and pre-update
 automatic snapshots, boot-menu snapshot restore, MAN Update and App
 Installer (.deb, Flatpak and AppImage files).
EOF
printf '%s\n' /etc/grub.d/41_man_update /etc/grub.d/42_man_snapshots /etc/grub.d/43_man_windows /etc/apt/apt.conf.d/80man-snapshot /etc/man/snapshots.conf /etc/xdg/gnome-mimeapps.list /etc/apt/sources.list.d/man-debian.sources /etc/apt/preferences.d/man-debian > "$r/DEBIAN/conffiles"
cat > "$r/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
if [ "$1" = configure ]; then
    systemctl disable man-boot-cleanup.service >/dev/null 2>&1 || true
    systemctl --global enable man-update-check.timer >/dev/null 2>&1 || true
    systemctl enable man-snapshot-daily.timer man-boot-cleanup.timer man-oobe-cleanup.service man-encrypt.service man-update-finish.service man-duhach-cleanup.service >/dev/null 2>&1 || true
    if [ -d /run/systemd/system ]; then
        systemctl daemon-reload || true
        systemctl start man-snapshot-daily.timer man-boot-cleanup.timer || true
    fi
    # The image was built with a local package folder as apt source; it does
    # not exist on MAN systems (Debian comes from man-debian.sources).
    if [ -f /etc/apt/sources.list ]; then
        sed -i 's|^deb \[trusted=yes\] copy:///.*|# &|' /etc/apt/sources.list
    fi
    update-mime-database /usr/share/mime 2>/dev/null || true
    update-desktop-database -q /usr/share/applications 2>/dev/null || true
    gtk-update-icon-cache -q -f /usr/share/icons/hicolor 2>/dev/null || true
    glib-compile-schemas /usr/share/glib-2.0/schemas 2>/dev/null || true
    # The MAN logo on the login screen (GDM's greeter dconf database).
    [ -x /usr/share/gdm/generate-config ] && /usr/share/gdm/generate-config >/dev/null 2>&1 || true
    # live-boot (needed by the live session package) puts the radeon/nouveau
    # drivers and ~500 firmware files into every initramfs; MAN's ISO has its
    # own start-up code, so its hooks are moved aside.
    mkdir -p /usr/lib/man/disabled-initramfs-hooks
    for h in kms live; do
        dpkg-divert --package man-system-tools --rename --quiet \
            --divert /usr/lib/man/disabled-initramfs-hooks/$h --add /usr/share/initramfs-tools/hooks/$h
    done
    # The MAN logo in Settings > About (and anything else reading os-release).
    if [ -f /usr/lib/os-release ] && ! grep -q "^LOGO=" /usr/lib/os-release; then
        echo "LOGO=man-logo" >> /usr/lib/os-release
    fi
    # Wallpaper thumbnails for MAN Setup, so its Customize page opens instantly.
    man-oobe --generate-thumbnails >/dev/null 2>&1 || true
    # On an installed MAN system, add the restore hook and boot entries now.
    if mountpoint -q /.snapshots 2>/dev/null && [ -f /boot/grub/grub.cfg ]; then
        update-initramfs -u -k all || true
        update-grub || true
    fi
fi
EOF
cat > "$r/DEBIAN/prerm" <<'EOF'
#!/bin/sh
set -e
if [ "$1" = remove ]; then
    systemctl disable --now man-snapshot-daily.timer man-boot-cleanup.timer >/dev/null 2>&1 || true
    systemctl --global disable man-update-check.timer >/dev/null 2>&1 || true
    for h in kms live; do
        dpkg-divert --package man-system-tools --rename --quiet --remove /usr/share/initramfs-tools/hooks/$h || true
    done
fi
EOF
chmod 0755 "$r/DEBIAN/postinst" "$r/DEBIAN/prerm"

# ── man-installer ───────────────────────────────────────────────────
r=$stage/man-installer
put "$bin/man-installer" "$r/usr/bin/man-installer" 0755
put "$data/applications/com.man.Installer.desktop" "$r/usr/share/applications/com.man.Installer.desktop" 0644
put "$data/autostart/com.man.Installer.desktop" "$r/etc/xdg/autostart/com.man.Installer.desktop" 0644
put "$data/metainfo/com.man.Installer.metainfo.xml" "$r/usr/share/metainfo/com.man.Installer.metainfo.xml" 0644
put "$data/icons/com.man.Installer.svg" "$r/usr/share/icons/hicolor/scalable/apps/com.man.Installer.svg" 0644
put "$data/polkit/49-man-live.rules" "$r/etc/polkit-1/rules.d/49-man-live.rules" 0644
copyright "$r"
mkdir -p "$r/DEBIAN"
cat > "$r/DEBIAN/control" <<EOF
Package: man-installer
Version: $version
Architecture: amd64
Maintainer: MAN Developers <dev@man.local>
Section: admin
Priority: optional
Depends: man-system-tools (= $version), libgtk-4-1 (>= 4.16), libadwaita-1-0 (>= 1.6)
Recommends: gnome-disk-utility
Description: MAN graphical installer
 Installs MAN from the live session: whole-disk or custom partitioning,
 LUKS2 encryption, and reinstalling while keeping /home. The system uses
 Btrfs with @system, @home and @snapshots subvolumes sharing free space.
EOF
printf '%s\n' /etc/xdg/autostart/com.man.Installer.desktop /etc/polkit-1/rules.d/49-man-live.rules > "$r/DEBIAN/conffiles"
cat > "$r/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
if [ "$1" = configure ]; then
    gtk-update-icon-cache -q -f /usr/share/icons/hicolor 2>/dev/null || true
fi
EOF
chmod 0755 "$r/DEBIAN/postinst"

# Translations: catalogs for both apps go into man-system-tools; the .desktop
# files of both packages get their Name[xx]/Comment[xx] lines.
python3 "$here/po/build-mo.py" "$stage/man-system-tools/usr/share/locale" $(find "$stage" -name '*.desktop')

for p in man-system-tools man-installer; do
    dpkg-deb --root-owner-group -Zxz -b "$stage/$p" "$out/${p}_${version}_amd64.deb" >/dev/null
    echo "built $out/${p}_${version}_amd64.deb"
done
