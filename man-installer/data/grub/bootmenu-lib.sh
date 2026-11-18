# MAN boot menu: what is on this computer (sourced by /etc/grub.d/4x_man_* and
# 49_man_bootmenu while update-grub runs). Installed as /usr/lib/man/grub/bootmenu-lib.sh.

ESP_GUID=c12a7328-f81f-11d2-ba4b-00a0c93ec93b

# man_windows: prints "<ESP UUID> <class> <title>" for Windows Boot Manager on
# an EFI system partition of this computer, e.g. "1234-ABCD windows11 Windows 11".
# The version comes from bootmgfw.efi's version resource (Windows keeps it
# current on every update): 10.0.22000 and later is Windows 11.
man_windows() {
    for esp in $(lsblk -nrpo PATH,PARTTYPE 2>/dev/null | awk -v g="$ESP_GUID" 'tolower($2) == g { print $1 }'); do
        mp=$(findmnt -nro TARGET -S "$esp" 2>/dev/null | head -n1)
        tmp=
        if [ -z "$mp" ]; then
            tmp=$(mktemp -d)
            if ! mount -o ro "$esp" "$tmp" >/dev/null 2>&1; then
                rmdir "$tmp"
                continue
            fi
            mp=$tmp
        fi
        ver=
        if [ -f "$mp/EFI/Microsoft/Boot/bootmgfw.efi" ]; then
            ver=$(man_pe_version "$mp/EFI/Microsoft/Boot/bootmgfw.efi")
            [ -n "$ver" ] || ver=unknown
        fi
        if [ -n "$tmp" ]; then
            umount "$tmp"
            rmdir "$tmp"
        fi
        if [ -n "$ver" ]; then
            uuid=$(blkid -s UUID -o value "$esp")
            echo "$uuid $(man_windows_name "$ver")"
            return 0
        fi
    done
    return 1
}

# File version (a.b.c.d) from a PE file's VS_FIXEDFILEINFO.
man_pe_version() {
    command -v python3 >/dev/null 2>&1 || return 0
    python3 -I - "$1" <<'PY' 2>/dev/null || true
import struct, sys
data = open(sys.argv[1], 'rb').read()
i = data.find(b'\xbd\x04\xef\xfe')
if i >= 0:
    ms, ls = struct.unpack_from('<II', data, i + 8)
    print(f"{ms >> 16}.{ms & 0xffff}.{ls >> 16}.{ls & 0xffff}")
PY
}

# "<class> <title>" for a Windows boot manager version.
man_windows_name() {
    major=${1%%.*}; rest=${1#*.}; minor=${rest%%.*}; rest=${rest#*.}; build=${rest%%.*}
    case "$major.$minor" in
        10.0) if [ "${build:-0}" -ge 22000 ] 2>/dev/null; then echo "windows11 Windows 11"; else echo "windows10 Windows 10"; fi ;;
        6.3) echo "windows8 Windows 8.1" ;;
        6.2) echo "windows8 Windows 8" ;;
        6.1) echo "windows7 Windows 7" ;;
        6.0) echo "windowsvista Windows Vista" ;;
        *) echo "windows Windows" ;;
    esac
}

# man_macos: on an Apple Silicon Mac whose start menu is on (Startup Disk),
# prints "<class> <title>", e.g. "macos-golden-gate macOS Golden Gate".
# The version was noted on the EFI partition by MAN's Mac installer.
man_macos() {
    [ -e /proc/device-tree/chosen/asahi,efi-system-partition ] || return 1
    [ -e /etc/default/grub.d/90-man-mac-menu.cfg ] || return 1
    ver=$(head -n1 /boot/efi/man/macos-version 2>/dev/null | tr -cd '0-9.')
    case "${ver%%.*}" in
        27) echo "macos-golden-gate macOS Golden Gate" ;;
        26) echo "macos-tahoe macOS Tahoe" ;;
        15) echo "macos-sequoia macOS Sequoia" ;;
        14) echo "macos-sonoma macOS Sonoma" ;;
        13) echo "macos-ventura macOS Ventura" ;;
        12) echo "macos-monterey macOS Monterey" ;;
        11) echo "macos-big-sur macOS Big Sur" ;;
        "") echo "macos macOS" ;;
        *) echo "macos macOS ${ver%%.*}" ;;
    esac
}

# man_recovery: MAN Recovery is set up (see 45_man_recovery).
man_recovery() {
    [ -f /var/lib/man/recovery ]
}
