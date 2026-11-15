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

rm -rf "$DST"; mkdir -p "$DST/os"
echo "copying $zip ..."
cp $R/$zip "$DST/os/"
cp $R/$inst "$DST/"
# Same OS entry as the online installer, but the package is the file in os/.
python3 -I - $R/installer_data.json "$zip" > "$DST/installer_data.json" <<'EOF'
import json, sys
d = json.load(open(sys.argv[1]))
for o in d["os_list"]:
    o["package"] = sys.argv[2]
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

Apple Silicon Macs cannot start from a USB stick. MAN is installed from macOS:

1. Back up your Mac (Time Machine). The installer can shrink macOS to make room.
2. Plug in this USB stick and connect to the internet (a small part of Apple's
   macOS files is downloaded from Apple during the installation).
3. Open Terminal (Applications > Utilities) and type:

       sh /Volumes/<name of this stick>/MAN-Mac/install-man.sh

   (Tip: type "sh ", then drag install-man.sh from Finder into the Terminal window.)
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
