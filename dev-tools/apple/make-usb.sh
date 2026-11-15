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
need=$(( $(stat -c %s $R/$zip) + $(stat -c %s $R/$inst) + 10*1024*1024 ))
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
python3 -I -c 'import sys, zipfile
with zipfile.ZipFile(sys.argv[2], "w", zipfile.ZIP_DEFLATED) as z:
    for f in ("esp/m1n1/boot.bin", "man.icns"):
        z.write(sys.argv[1] + "/" + f, f)' $B $R/$bl
rm -rf $B

rm -rf "$DST"; mkdir -p "$DST/os"
echo "copying $zip ..."
cp $R/$zip $R/$bl "$DST/os/"
cp $R/$inst "$DST/"
# Same OS entry as the online installer, but the package is the file in os/.
python3 -I - $R/installer_data.json "$zip" "$bl" > "$DST/installer_data.json" <<'EOF'
import copy, json, sys
d = json.load(open(sys.argv[1]))
full = d["os_list"][0]
full["package"] = sys.argv[2]
bl = copy.deepcopy(full)
bl.update(name="MAN boot loader only (to start the MAN Live USB stick)",
          default_os_name="MAN Live USB", package=sys.argv[3])
bl["partitions"] = [p for p in full["partitions"] if p["type"] == "EFI"]
bl["partitions"][0].pop("volume_id", None)
d["os_list"] = [full, bl]
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
    export REPO_BASE="\$USB"
    unset REPORT REPORT_TAG
    TMP=/tmp/man-install
    echo
    echo "MAN for Apple Silicon Macs (from the USB stick)"
    [ -e "\$TMP" ] && mv "\$TMP" "\$TMP-\$(date +%Y%m%d-%H%M%S)"
    mkdir -p "\$TMP"; cd "\$TMP"
    echo "  Extracting the installer..."
    tar xf "\$USB/$inst"
    cp "\$USB/installer_data.json" .
    echo "  Starting (keep the USB stick plugged in until the installer is done)..."
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

An Apple Silicon Mac starts this stick only after a one-time step in macOS.

Start the MAN Live System from this stick
-----------------------------------------
Once per Mac, macOS has to register MAN's boot loader (Apple only lets macOS do this):
1. In macOS, open Terminal and type (with the stick plugged in):
       sh /Volumes/MAN_LIVE/MAN-Mac/install-man.sh
2. Choose "MAN boot loader only". It needs about 3 GB of free space for a small
   macOS helper and the boot loader; it does not install MAN.
3. Follow the installer to the end (it asks you to start in recovery mode once).
From then on: plug in this stick, hold the power button at start-up, choose
"MAN Live USB": the MAN Live System starts from the stick.

Install MAN on the Mac
----------------------
1. Back up your Mac (Time Machine). The installer can shrink macOS to make room.
2. Plug in this USB stick and connect to the internet (a small part of Apple's
   macOS files is downloaded from Apple during the installation).
3. Open Terminal (Applications > Utilities) and type:

       sh /Volumes/MAN_LIVE/MAN-Mac/install-man.sh

   (Tip: type "sh ", then drag install-man.sh from Finder into the Terminal window.)
   Choose "MAN 10 ..." (not "boot loader only").
4. Follow the installer. At the end it asks you to shut down and start in
   recovery mode once (hold the power button) to confirm MAN. Follow its steps.
5. MAN starts and MAN Setup appears. Later, hold the power button at start-up to
   choose between macOS and MAN.

The installer is the Asahi Linux installer (https://asahilinux.org, MIT licence).
M4 and newer Macs are not supported yet.
EOF
cp $R/$zip.sha256 "$DST/os/"
sync
echo "checking the copy on the stick ..."
(cd "$DST/os" && sha256sum -c --quiet $zip.sha256 && echo "package verified on the stick")
sync
ls -laR "$DST" | grep -v '^total'
