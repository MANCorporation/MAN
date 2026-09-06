#!/usr/bin/env bash
# Build and install the pinned COSMIC desktop release into a MAN rootfs.
#
# COSMIC Epoch is a superproject containing independent Git submodules, not a
# single Cargo workspace. Each component must be built from its own manifest.

set -euo pipefail

ARCH="${1:-aarch64}"
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUTPUT_DIR="${2:-$PROJECT_ROOT/output/$ARCH}"
COSMIC_VERSION="${COSMIC_VERSION:-epoch-1.6.0}"
if [ -d "$PROJECT_ROOT/Desktop" ]; then
    COSMIC_DIR="$PROJECT_ROOT/Desktop"
else
    COSMIC_DIR="$OUTPUT_DIR/build/cosmic-epoch"
fi
GREETD_DIR="$OUTPUT_DIR/build/greetd"
GREETD_VERSION="0.10.3"
XKB_DIR="$OUTPUT_DIR/build/xkeyboard-config"
XKB_VERSION="xkeyboard-config-2.38"
TARGET_DIR="$OUTPUT_DIR/target"
HOST_DIR="$OUTPUT_DIR/host"
COSMIC_TARGET_DIR="$OUTPUT_DIR/build/cosmic-target"

case "$ARCH" in
    aarch64)
        GNU_TRIPLE="aarch64-buildroot-linux-gnu"
        RUST_TRIPLE="aarch64-unknown-linux-gnu"
        ;;
    x86_64)
        GNU_TRIPLE="x86_64-buildroot-linux-gnu"
        RUST_TRIPLE="x86_64-unknown-linux-gnu"
        ;;
    *)
        echo "error: unsupported architecture '$ARCH' (expected aarch64 or x86_64)" >&2
        exit 2
        ;;
esac

SYSROOT="$HOST_DIR/$GNU_TRIPLE/sysroot"

require_file() {
    [ -e "$1" ] || { echo "error: required file is missing: $1" >&2; exit 1; }
}

require_file "$HOST_DIR/bin/$GNU_TRIPLE-gcc"
require_file "$SYSROOT/usr/lib/pkgconfig"
[ -d "$TARGET_DIR" ] || {
    echo "error: MAN rootfs is not built at $TARGET_DIR; run 'man build --arch $ARCH' first" >&2
    exit 1
}
CARGO_BIN="${CARGO:-$(command -v cargo || true)}"
RUSTUP_BIN="$(command -v rustup || true)"
[ -n "$CARGO_BIN" ] || { echo "error: cargo is required" >&2; exit 1; }
[ -n "$RUSTUP_BIN" ] || { echo "error: rustup is required" >&2; exit 1; }

export PATH="$HOST_DIR/bin:$PATH"
export RUSTC="${RUSTC:-$($RUSTUP_BIN which rustc)}"
export RUSTDOC="${RUSTDOC:-$($RUSTUP_BIN which rustdoc)}"
export PKG_CONFIG_ALLOW_CROSS=1
export PKG_CONFIG_SYSROOT_DIR="$SYSROOT"
export PKG_CONFIG_PATH="$SYSROOT/usr/lib/pkgconfig:$SYSROOT/usr/share/pkgconfig"
export BINDGEN_EXTRA_CLANG_ARGS="--target=$RUST_TRIPLE --sysroot=$SYSROOT -I$SYSROOT/usr/include"
export "CC_${RUST_TRIPLE//-/_}=$GNU_TRIPLE-gcc"
export "CXX_${RUST_TRIPLE//-/_}=$GNU_TRIPLE-g++"
export "AR_${RUST_TRIPLE//-/_}=$GNU_TRIPLE-ar"
export COSMIC_STRIP="$HOST_DIR/bin/$GNU_TRIPLE-strip"
export COSMIC_CC="$HOST_DIR/bin/$GNU_TRIPLE-gcc"

# pam-sys uses bindgen. On macOS Homebrew's libclang is used by bindgen.
# Do NOT export DYLD_LIBRARY_PATH as it will cause rustc to dynamically link
# against Homebrew's LLVM dylibs and crash with SIGSEGV due to ABI mismatch.
if [ "$(uname -s)" = Darwin ]; then
    for llvm_lib in \
        "$HOME/.homebrew/opt/llvm@21/lib" \
        "$HOME/.homebrew/opt/llvm/lib" \
        /opt/homebrew/opt/llvm@21/lib \
        /opt/homebrew/opt/llvm/lib \
        /usr/local/opt/llvm/lib; do
        if [ -f "$llvm_lib/libclang.dylib" ]; then
            export LIBCLANG_PATH="$llvm_lib"
            break
        fi
    done
fi

# Cargo environment variable names use underscores, not dashes.
linker_target="$(printf '%s' "$RUST_TRIPLE" | tr '[:lower:]-' '[:upper:]_')"
linker_var="CARGO_TARGET_${linker_target}_LINKER"
export "$linker_var=$GNU_TRIPLE-gcc"
export RUSTFLAGS="${RUSTFLAGS:-} -C link-arg=--sysroot=$SYSROOT -L $SYSROOT/usr/lib -L $SYSROOT/lib"

if ! "$RUSTUP_BIN" target list --installed | grep -qx "$RUST_TRIPLE"; then
    "$RUSTUP_BIN" target add "$RUST_TRIPLE"
fi

# libxkbcommon needs the standard keyboard database at runtime. Keep this data
# independent of Buildroot's X.Org menu so the native Wayland image does not
# pull in an X server and its build-time dependency graph.
if [ ! -d "$XKB_DIR/.git" ]; then
    git clone --branch "$XKB_VERSION" --depth 1 \
        https://gitlab.freedesktop.org/xkeyboard-config/xkeyboard-config.git "$XKB_DIR"
else
    git -C "$XKB_DIR" checkout --detach "$XKB_VERSION"
fi

echo "=== COSMIC $COSMIC_VERSION for MAN/$ARCH (Source: $COSMIC_DIR) ==="
if [ "$COSMIC_DIR" != "$PROJECT_ROOT/Desktop" ]; then
    if [ ! -d "$COSMIC_DIR/.git" ]; then
        git clone --branch "$COSMIC_VERSION" --depth 1 --recurse-submodules \
            --shallow-submodules https://github.com/pop-os/cosmic-epoch.git "$COSMIC_DIR"
    else
        if [ "$(git -C "$COSMIC_DIR" describe --tags --exact-match 2>/dev/null || true)" != "$COSMIC_VERSION" ]; then
            git -C "$COSMIC_DIR" fetch --tags --force origin "$COSMIC_VERSION"
            git -C "$COSMIC_DIR" checkout --detach "$COSMIC_VERSION"
        fi
        git -C "$COSMIC_DIR" submodule update --init --recursive
    fi
fi

# Add MAN's in-process Flatpak permissions page as a first-class COSMIC
# Settings page. It stays as a small, reviewable patch on the pinned release.
SETTINGS_PRIVACY_PATCH="$PROJECT_ROOT/patches/cosmic-settings-privacy-security.patch"
if git -C "$COSMIC_DIR/cosmic-settings" apply --check "$SETTINGS_PRIVACY_PATCH" 2>/dev/null; then
    git -C "$COSMIC_DIR/cosmic-settings" apply "$SETTINGS_PRIVACY_PATCH"
fi

# App Depot is pinned as a Flathub-only frontend. Apply its source patch before
# any component build so the focused rebuild mode below compiles the same code
# that a full desktop build uses.
APP_DEPOT_PATCH="$PROJECT_ROOT/patches/cosmic-store-app-depot.patch"
if git -C "$COSMIC_DIR/cosmic-store" apply --check "$APP_DEPOT_PATCH" 2>/dev/null; then
    git -C "$COSMIC_DIR/cosmic-store" apply "$APP_DEPOT_PATCH"
fi

build_component() {
    component="$1"
    shift
    echo "  -> building $component"
    CARGO_TARGET_DIR="$COSMIC_TARGET_DIR" \
    "$CARGO_BIN" build --locked --release --target "$RUST_TRIPLE" \
        --manifest-path "$COSMIC_DIR/$component/Cargo.toml" "$@"
}

# Rebuild just Settings when a Settings-only source change is being iterated.
# The page patch above has already been applied to the source tree at this
# point; this mode compiles and installs the normal COSMIC package, it does
# not modify the finished desktop or its files afterwards.
if [ "${MAN_SETTINGS_ONLY:-0}" = 1 ]; then
    build_component cosmic-settings
    "$PROJECT_ROOT/scripts/install-cosmic.sh" \
        "$COSMIC_DIR" "$RUST_TRIPLE" "$TARGET_DIR" "$COSMIC_VERSION" "$XKB_DIR" "$GREETD_DIR"
    exit 0
fi

if [ "${MAN_APP_DEPOT_ONLY:-0}" = 1 ]; then
    echo "  -> building App Depot"
    CARGO_TARGET_DIR="$COSMIC_TARGET_DIR" \
        "$CARGO_BIN" build --locked --release --target "$RUST_TRIPLE" \
        --manifest-path "$COSMIC_DIR/cosmic-store/Cargo.toml" \
        --no-default-features \
        --features flatpak,wayland,wgpu,xdg-portal,single-instance,dbus-config,desktop
    "$PROJECT_ROOT/scripts/install-cosmic.sh" \
        "$COSMIC_DIR" "$RUST_TRIPLE" "$TARGET_DIR" "$COSMIC_VERSION" "$XKB_DIR" "$GREETD_DIR"
    exit 0
fi

if [ "${MAN_UTILITIES_ONLY:-0}" != 1 ]; then
# MAN uses BusyBox init and libseat's builtin backend, so do not link the
# compositor/session to systemd or logind.
build_component cosmic-comp --no-default-features
# Epoch 1.6 accidentally gates the is_systemd_used import behind its systemd
# feature even though the non-systemd autostart path calls it. Carry the tiny
# upstreamable Rust fix and build a genuinely systemd/logind-free session.
SESSION_PATCH="$PROJECT_ROOT/patches/cosmic-session-busybox.patch"
if git -C "$COSMIC_DIR/cosmic-session" apply --check "$SESSION_PATCH" 2>/dev/null; then
    git -C "$COSMIC_DIR/cosmic-session" apply "$SESSION_PATCH"
fi
build_component cosmic-session --no-default-features --features autostart
build_component cosmic-settings-daemon
build_component cosmic-notifications
build_component cosmic-panel
build_component cosmic-applets
build_component cosmic-applibrary
build_component cosmic-launcher
build_component cosmic-workspaces-epoch
build_component cosmic-osd
# MAN supports the normal PNG/JPEG/WebP/JXL wallpaper formats without pulling
# the optional native dav1d AVIF decoder into the minimal cross sysroot.
build_component cosmic-bg --no-default-features
build_component cosmic-idle
# pam-client 0.5.0 assumes bindgen emits signed PAM constants. Keep MAN's
# architecture-neutral casts in a vendored crate until that upstream release
# supports the unsigned constants emitted for aarch64.
GREETER_PAM_PATCH="$PROJECT_ROOT/patches/cosmic-greeter-pam-client.patch"
if git -C "$COSMIC_DIR/cosmic-greeter" apply --check "$GREETER_PAM_PATCH" 2>/dev/null; then
    git -C "$COSMIC_DIR/cosmic-greeter" apply "$GREETER_PAM_PATCH"
fi
GREETER_BUSYBOX_PATCH="$PROJECT_ROOT/patches/cosmic-greeter-busybox.patch"
if git -C "$COSMIC_DIR/cosmic-greeter" apply --check "$GREETER_BUSYBOX_PATCH" 2>/dev/null; then
    git -C "$COSMIC_DIR/cosmic-greeter" apply "$GREETER_BUSYBOX_PATCH"
fi
GREETER_IMAGE_PATCH="$PROJECT_ROOT/patches/cosmic-greeter-no-avif.patch"
if git -C "$COSMIC_DIR/cosmic-greeter" apply --check "$GREETER_IMAGE_PATCH" 2>/dev/null; then
    git -C "$COSMIC_DIR/cosmic-greeter" apply "$GREETER_IMAGE_PATCH"
fi
echo "  -> building cosmic-greeter"
# Cargo must record the local crate replacement once after applying the patch.
CARGO_TARGET_DIR="$COSMIC_TARGET_DIR" \
"$CARGO_BIN" build --release --target "$RUST_TRIPLE" \
    --manifest-path "$COSMIC_DIR/cosmic-greeter/Cargo.toml" --no-default-features
build_component cosmic-greeter/daemon --no-default-features

# COSMIC Greeter speaks greetd's IPC protocol. Pin the matching official
# daemon so authentication and privilege dropping are reproducible.
if [ ! -d "$GREETD_DIR/.git" ]; then
    git clone --branch "$GREETD_VERSION" --depth 1 \
        https://github.com/kennylevinsen/greetd.git "$GREETD_DIR"
else
    git -C "$GREETD_DIR" checkout --detach "$GREETD_VERSION"
fi
RUSTFLAGS="${RUSTFLAGS:-} -C link-arg=-lpam_misc" \
"$CARGO_BIN" build --locked --release --target "$RUST_TRIPLE" \
    --manifest-path "$GREETD_DIR/greetd/Cargo.toml"

# Useful first-party applications.
# MAN Guide is launched through COSMIC Terminal, so this component is a
# required part of the desktop image rather than an optional utility.
build_component cosmic-term

OPTIONAL_COMPONENTS="cosmic-settings cosmic-files cosmic-monitor cosmic-randr cosmic-screenshot"
if [ "${MAN_SKIP_OPTIONAL:-0}" != 1 ]; then
for component in $OPTIONAL_COMPONENTS; do
    # GVFS pulls in GLib/GIO, which MAN intentionally does not otherwise need.
    # Files remains fully native Rust for local files, archives, D-Bus,
    # notifications and Wayland by selecting every default except GVFS.
    if [ "$component" = cosmic-files ]; then
        component_args="--no-default-features --features bzip2,dbus-config,desktop,io-uring,lzma-rust2,notify,wayland,wgpu"
    else
        component_args=""
    fi
    # shellcheck disable=SC2086 # component_args is an intentional Cargo argv list.
    if ! build_component "$component" $component_args; then
        echo "warning: optional COSMIC component failed: $component" >&2
    fi
done
fi

# App Depot is MAN's Flatpak-only application store. Build the pinned COSMIC
# Store frontend without PackageKit, logind, or systemd integration. Buildroot
# exposes libflatpak through the cross sysroot when FLATPAK_INSTALL_STAGING is
# enabled in MAN's package recipe.
echo "  -> building App Depot"
CARGO_TARGET_DIR="$COSMIC_TARGET_DIR" \
    "$CARGO_BIN" build --locked --release --target "$RUST_TRIPLE" \
    --manifest-path "$COSMIC_DIR/cosmic-store/Cargo.toml" \
    --no-default-features \
    --features flatpak,wayland,wgpu,xdg-portal,single-instance,dbus-config,desktop
fi

# MAN's early splash is dependency-free Rust. All COSMIC components and MAN's
# libcosmic applications share one architecture-specific target directory, so
# common dependencies are compiled once instead of consuming tens of gigabytes
# in a separate cache for every component.
echo "  -> building MAN splash and graphical Utilities"
"$CARGO_BIN" build --release --target "$RUST_TRIPLE" \
    --manifest-path "$PROJECT_ROOT/support/man-splash/Cargo.toml"
CARGO_TARGET_DIR="$COSMIC_TARGET_DIR" \
    "$CARGO_BIN" build --release --target "$RUST_TRIPLE" \
    --manifest-path "$PROJECT_ROOT/support/man-utilities/Cargo.toml"

echo "  -> installing COSMIC binaries and data"
"$PROJECT_ROOT/scripts/install-cosmic.sh" \
    "$COSMIC_DIR" "$RUST_TRIPLE" "$TARGET_DIR" "$COSMIC_VERSION" "$XKB_DIR" "$GREETD_DIR"

echo "=== COSMIC installation complete ==="
