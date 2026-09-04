#!/bin/bash
# Post-build script for MAN Linux Distribution.
# Called by Toolchain after the root filesystem has been assembled.
#
# Args:
#   $1 = target directory (the rootfs staging area)
#   $2 = target architecture (e.g. "x86_64" or "aarch64")
#
# Environment (provided by Toolchain):
#   $HOST_DIR  — host directory with cross-compilation tools
#   $TARGET_DIR — same as $1

set -e

TARGET_DIR="$1"
HOST_DIR="${HOST_DIR:-$(dirname "$0")/../output/host}"

echo "=== MAN post-build script ==="
echo "Target dir: $TARGET_DIR"
echo "Host dir:   $HOST_DIR"

# --- Ensure MAN branding files are correct ---
echo "  -> Installing MAN branding files"

# Ensure hostname
echo "man" > "${TARGET_DIR}/etc/hostname"

# Ensure os-release is correct
cat > "${TARGET_DIR}/etc/os-release" << 'OSRELEASE'
NAME="MAN"
ID=man
ID_LIKE=linux
PRETTY_NAME="MAN Linux Distribution"
ANSI_COLOR="0;36"
HOME_URL="https://github.com/kristihack/MAN"
VERSION="0.1.0 (alpha)"
VERSION_ID="0.1.0"
BUILD_ID="alpha"
OSRELEASE

# Ensure MOTD
if [ -f "${TARGET_DIR}/etc/motd" ]; then
    rm -f "${TARGET_DIR}/etc/motd"
fi
# The MOTD is in the overlay which gets merged into TARGET_DIR
if [ -f "${TARGET_DIR}/etc/motd" ]; then
    echo "  [OK] MOTD installed"
fi

# Buildroot invokes post-build scripts with only TARGET_DIR. Infer the target
# from its output directory when no explicit second argument is supplied;
# uname reports the macOS host and is wrong for cross-architecture builds.
TARGET_ARCH="${2:-${BR2_ARCH:-${BR2_TARGET_ARCH:-$(basename "$(dirname "$TARGET_DIR")")}}}"
case "$TARGET_ARCH" in
    x86_64|x86) TARGET_TRIPLE="x86_64-unknown-linux-gnu" ;;
    arm64|aarch64) TARGET_TRIPLE="aarch64-unknown-linux-gnu" ;;
    *)          TARGET_TRIPLE="$TARGET_ARCH-unknown-linux-gnu" ;;
esac
echo "  -> Target triple: ${TARGET_TRIPLE}"

# --- Check for host Rust toolchain ---
HAS_RUST="no"
if [ -x "${HOST_DIR}/usr/bin/cargo" ] && "${HOST_DIR}/usr/bin/cargo" --version >/dev/null 2>&1; then
    export PATH="${HOST_DIR}/usr/bin:${PATH}"
    export CARGO_HOME="${HOST_DIR}/usr/bin"
    export RUSTUP_HOME="${HOST_DIR}/usr/bin/rustlib"
    HAS_RUST="yes"
    echo "  -> Host Rust toolchain available"
elif command -v cargo &>/dev/null; then
    HAS_RUST="yes"
    echo "  -> System cargo available"
fi

# --- Install manctl (Rust utility) ---
MANCTL_SRC="${TARGET_DIR}/usr/src/manctl"
if [ -d "${MANCTL_SRC}" ]; then
    echo "  -> Found manctl source, building for ${TARGET_TRIPLE}"
    if [ "$HAS_RUST" = "yes" ]; then
        echo "  -> Building manctl for ${TARGET_TRIPLE}..."
        if cargo build --release --manifest-path "${MANCTL_SRC}/Cargo.toml" --target "${TARGET_TRIPLE}" 2>&1; then
            cp "${MANCTL_SRC}/target/${TARGET_TRIPLE}/release/manctl" "${TARGET_DIR}/usr/bin/manctl"
            chmod 0755 "${TARGET_DIR}/usr/bin/manctl"
            echo "  [OK] manctl installed to /usr/bin/manctl"
        else
            echo "  [WARN] manctl build failed, source left at /usr/src/manctl"
        fi
    else
        echo "  [WARN] Rust toolchain not available, manctl source left at /usr/src/manctl"
    fi
fi

# --- Build uutils/coreutils (Rust reimplementation of GNU coreutils) ---
UUTILS_DIR="${TARGET_DIR}/usr/src/uutils"
if [ ! -d "${UUTILS_DIR}" ] && [ "$HAS_RUST" = "yes" ]; then
    echo "  -> Cloning uutils/coreutils..."
    git clone --depth 1 https://github.com/uutils/coreutils.git "${UUTILS_DIR}" 2>/dev/null || true
fi

if [ -d "${UUTILS_DIR}" ] && [ -f "${UUTILS_DIR}/Cargo.toml" ] && [ "$HAS_RUST" = "yes" ]; then
    echo "  -> Building uutils for ${TARGET_TRIPLE}..."
    if cargo build --release --manifest-path "${UUTILS_DIR}/Cargo.toml" --target "${TARGET_TRIPLE}" 2>&1; then
        UUTILS_BIN_DIR="${UUTILS_DIR}/target/${TARGET_TRIPLE}/release/"
        for bin in "${UUTILS_BIN_DIR}"/*; do
            if [ -x "$bin" ] && [ ! -L "$bin" ]; then
                local_bin_name=$(basename "$bin")
                # Don't overwrite busybox symlinks
                if [ ! -e "${TARGET_DIR}/usr/bin/${local_bin_name}" ]; then
                    cp "$bin" "${TARGET_DIR}/usr/bin/${local_bin_name}"
                    chmod 0755 "${TARGET_DIR}/usr/bin/${local_bin_name}"
                fi
            fi
        done
        echo "  [OK] uutils coreutils installed to /usr/bin/"
    else
        echo "  [WARN] uutils build failed, source left at /usr/src/uutils"
    fi
else
    if [ "$HAS_RUST" != "yes" ]; then
        echo "  [WARN] Rust toolchain not available, skipping uutils compilation"
    fi
fi

# --- Ensure /usr/local dirs exist for Rust toolchain ---
mkdir -p "${TARGET_DIR}/usr/local/bin"
mkdir -p "${TARGET_DIR}/usr/local/cargo"
mkdir -p "${TARGET_DIR}/usr/local/rustup"

# --- Create MAN-specific directories ---
mkdir -p "${TARGET_DIR}/usr/share/man-distro"
echo "MAN Linux Distribution v0.1.0" > "${TARGET_DIR}/usr/share/man-distro/version"
echo "Build system: Toolchain + Rust" > "${TARGET_DIR}/usr/share/man-distro/description"

# --- Generate system locales for 50+ supported languages ---
# This enables proper locale rendering in COSMIC, the shell, and other
# glibc-based system components.  man-utilities translations are loaded
# from JSON files independently and do not depend on this step.
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
if [ -x "${PROJECT_ROOT}/scripts/generate-locales.sh" ]; then
    "${PROJECT_ROOT}/scripts/generate-locales.sh" "$TARGET_DIR" "$HOST_DIR" || \
        echo "  [WARN] Locale generation skipped (localedef unavailable)"
fi

# --- Clean up large caches to reduce image size ---
rm -rf "${TARGET_DIR}/usr/share/doc" 2>/dev/null || true
rm -rf "${TARGET_DIR}/usr/share/man" 2>/dev/null || true
rm -rf "${TARGET_DIR}/var/cache/*" 2>/dev/null || true
rm -rf "${TARGET_DIR}/root/.cache" 2>/dev/null || true

# --- Install MAN and third-party licensing notices ---
# Keep these after the documentation cleanup so both the installed system and
# the installation payload contain the notices required for redistribution.
LEGAL_DOC_DIR="${TARGET_DIR}/usr/share/doc/MAN"
mkdir -p "${LEGAL_DOC_DIR}"
install -m 0644 "${PROJECT_ROOT}/LICENSE" "${LEGAL_DOC_DIR}/LICENSE"
install -m 0644 "${PROJECT_ROOT}/LICENSES/LicenseRef-MAN-Proprietary.txt" \
    "${LEGAL_DOC_DIR}/MAN-PROPRIETARY-LICENSE.txt"
install -m 0644 "${PROJECT_ROOT}/THIRD_PARTY_NOTICES.md" \
    "${LEGAL_DOC_DIR}/THIRD_PARTY_NOTICES.md"
install -m 0644 "${PROJECT_ROOT}/DISTRIBUTION_COMPLIANCE.md" \
    "${LEGAL_DOC_DIR}/DISTRIBUTION_COMPLIANCE.md"
install -m 0644 "${PROJECT_ROOT}/ASSET_PROVENANCE.md" \
    "${LEGAL_DOC_DIR}/ASSET_PROVENANCE.md"
install -m 0644 "${PROJECT_ROOT}/scripts/bootloader/THIRD_PARTY-ICON-LICENSES.md" \
    "${LEGAL_DOC_DIR}/rEFInd-ICON-NOTICES.md"
install -m 0644 "${PROJECT_ROOT}/support/pam-client/LICENSE" \
    "${LEGAL_DOC_DIR}/MPL-2.0-pam-client.txt"

# --- Set permissions ---
chmod 0600 "${TARGET_DIR}/etc/hostname" 2>/dev/null || true
chmod 0644 "${TARGET_DIR}/etc/os-release" 2>/dev/null || true

echo "  [OK] MAN post-build complete"
