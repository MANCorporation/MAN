#!/bin/bash
# MAN installer for Apple Silicon Macs on a USB stick (or any folder), from the files
# make-package.sh put in release/apple. Apple Silicon Macs cannot start from USB: the
# stick is used from macOS ("sh /Volumes/<stick>/MAN-Mac/install-man.sh"), and the
# Asahi installer then reads MAN's package from the stick instead of downloading it.
# Existing files on the stick are left alone; only <dir>/MAN-Mac is (re)written.
#   make-usb.sh /media/$USER/<STICK>
set -euo pipefail
p=$(cd -- "$(dirname -- "$(readlink -f "$0")")/../.." && pwd)
R=$p/release/apple; DST=${1:?mounted USB stick (or folder)}/MAN-Mac
zip=$(cd $R && ls MAN-*-apple.zip | head -n1)
inst=$(cd $R && ls installer-*.tar.gz | head -n1)
need=$(( $(stat -c %s $R/$inst) + 20*1024*1024 ))
free=$(df -B1 --output=avail "$1" | tail -1)
[ "$free" -gt "$need" ] || { echo "Not enough space on $1"; exit 1; }

# "MAN boot loader only": the one-time setup for starting the MAN Live USB stick on a
# Mac. Only the EFI partition with m1n1 stage 2 + device trees + U-Boot, no GRUB and no
# system, so U-Boot (boot order nvme, usb) goes on to the USB stick.
A=$p/lfs/apple; bl=${zip%.zip}-bootloader.zip
m1=$A/debs/m1n1/usr/lib/m1n1/m1n1.bin
[ -f $m1 ] || { mkdir -p $A/debs/m1n1; dpkg-deb -x $A/debs/m1n1_*_arm64.deb $A/debs/m1n1; }
B=$(mktemp -d); mkdir -p $B/esp/m1n1
d=$A/kernel/dtbs/apple
{ cat $m1 $d/t6*.dtb $d/t81*.dtb; gzip -9 -n -c $A/ub/u-boot-nodtb.bin; } > $B/esp/m1n1/boot.bin
cp $p/dev-tools/apple/man.icns $B/
rm -f $R/$bl
# Folder entries are needed: the Asahi installer only creates folders it finds in the zip.
python3 -I -c 'import sys, zipfile
with zipfile.ZipFile(sys.argv[2], "w", zipfile.ZIP_DEFLATED) as z:
    for f in ("esp/", "esp/m1n1/", "esp/m1n1/boot.bin", "man.icns"):
        z.write(sys.argv[1] + "/" + f, f)' $B $R/$bl
rm -rf $B

rm -rf "$DST"; mkdir -p "$DST/os"
cp $R/$bl "$DST/os/"
cp $R/$inst "$DST/"
# Same OS entry as the online installer, but the package is the file in os/.
python3 -I - $R/installer_data.json "$zip" "$bl" > "$DST/installer_data.json" <<'EOF'
import copy, json, sys
d = json.load(open(sys.argv[1]))
full = d["os_list"][0]
full["package"] = sys.argv[2]
bl = copy.deepcopy(full)
bl.update(name="MAN boot loader only (to start the MAN Live USB stick)",
          default_os_name="MAN", package=sys.argv[3], man_mode="1")
bl["partitions"] = [p for p in full["partitions"] if p["type"] == "EFI"]
bl["partitions"][0].pop("volume_id", None)
# Boot loader + empty partitions made by macOS's own diskutil: MAN's live USB
# installer then only formats these two (it never touches macOS's partitions).
space = copy.deepcopy(bl)
space.update(name="MAN boot loader + space for MAN (install MAN later from the MAN Live USB)",
             default_os_name="MAN", man_mode="2")
space["partitions"] = bl["partitions"] + [
    {"name": "MAN Boot", "type": "Linux", "size": "1073741824B"},
    {"name": "MAN", "type": "Linux", "size": "20000000000B", "expand": True},
]
d["os_list"] = [space, bl]
print(json.dumps(d, indent=1))
EOF
cat > "$DST/install-man.sh" <<EOF
#!/bin/sh
# SPDX-License-Identifier: MIT
# MAN for Apple Silicon Macs, from this USB stick. In the macOS Terminal:
#   sh /Volumes/<name of the stick>/MAN-Mac/install-man.sh
# Based on the Asahi Linux installer's bootstrap (MIT, The Asahi Linux Contributors).
if true; then
    set -e
    if [ ! -e /System ]; then
        echo "Run this in the Terminal of macOS on your Apple Silicon Mac."
        exit 1
    fi
    if [ "\$(uname -m)" != "arm64" ]; then
        echo "This Mac has an Intel processor: use the normal MAN ISO instead."
        exit 1
    fi
    export LC_ALL=en_US.UTF-8 LANG=en_US.UTF-8
    export PATH="/usr/bin:/bin:/usr/sbin:/sbin:\$PATH"
    USB="\$(cd "\$(dirname "\$0")" && pwd)"
    export REPO_BASE="\$USB" DISTRO=MAN MAN_AUTO=1
    unset REPORT REPORT_TAG
    TMP=/tmp/man-install
    echo
    echo "MAN for Apple Silicon Macs"
    [ -e "\$TMP" ] && mv "\$TMP" "\$TMP-\$(date +%Y%m%d-%H%M%S)"
    mkdir -p "\$TMP"; cd "\$TMP"
    echo "  Extracting the installer..."
    tar xf "\$USB/$inst"
    cp "\$USB/installer_data.json" .
    echo "  Starting (keep this window open until the installer is done)..."
    echo
    if [ "\$USER" != "root" ]; then
        echo "The installer needs to run as root. Enter your macOS password if asked."
        exec caffeinate -dis sudo -E ./install.sh "\$@"
    else
        exec caffeinate -dis ./install.sh "\$@"
    fi
fi
EOF
cat > "$DST/README.txt" <<EOF
MAN for Apple Silicon Macs (M1, M2, M3)
=======================================

macOS is never changed: the installer only adds MAN's own small partitions next to it
(with Apple's diskutil) and macOS keeps Full Security. Back up with Time Machine first
anyway, and keep the charger plugged in.

Easiest: in macOS, open this stick (MAN_MAC) and double-click DUHACH!.app. It asks
the questions below in simple windows and guides you through each step.

1. Or by hand: in macOS, plug in this stick, connect to the internet, open Terminal and type:
       sh /Volumes/MAN_MAC/MAN-Mac/install-man.sh
   and enter your macOS password. Choose:
     1  Install only the MAN boot loader (to start the MAN Live USB stick)
     2  Install the MAN boot loader and make a partition for MAN
     3  Make a partition for MAN (when the boot loader is already installed)
   For 2 and 3 type the size for MAN (for example 60GB). The space comes from
   free space on the disk or from macOS's free space; macOS keeps at least 38 GB free.
2. After 1 or 2 the Mac shuts down by itself. Then press and HOLD the power button
   until "Loading startup options" appears, and choose "MAN" (Apple requires this
   one step in person). Type your macOS password when asked; the Mac restarts by
   itself.
3. Start the MAN Live System: plug in this stick, hold the power button, choose
   MAN. In the live system, "Install MAN" offers "Install on This Mac" (the
   partition from 2 or 3) or an external disk; macOS's disk is not even shown.
4. To start macOS by default again: System Settings > General > Startup Disk.

The installer is the Asahi Linux installer (https://asahilinux.org, MIT licence),
with MAN's changes. M4 and newer Macs are not supported yet.
EOF
(cd $R && sha256sum $bl) > "$DST/os/$bl.sha256"
sync
echo "checking the copy on the stick ..."
(cd "$DST/os" && sha256sum -c --quiet $bl.sha256 && echo "package verified on the stick")
sync
ls -laR "$DST" | grep -v '^total'
