#!/bin/bash
# DUHACH! for Mac: release/DUHACH!-<ver>-mac.zip with DUHACH!.app inside.
# The app = this folder's guide (AppleScript, run by osascript: no compiling, no Mac
# needed to build) + MAN-Mac (MAN's copy of the Asahi installer and MAN's boot loader
# package, from dev-tools/apple/make-usb.sh). make-iso.sh also puts the app on the
# MAN_MAC partition of the Mac live USB stick.
#   build-mac.sh [OUTDIR]   (default: release/, zip; with OUTDIR: just DUHACH!.app there)
set -euo pipefail
here=$(cd -- "$(dirname -- "$(readlink -f "$0")")" && pwd)
p=$(cd -- "$here/../.." && pwd)
VER=1.0.0
W=$(mktemp -d); trap 'rm -rf "$W"' EXIT
A="$W/DUHACH!.app/Contents"
mkdir -p "$A/MacOS" "$A/Resources"
sed "s/@VERSION@/$VER/" "$here/Info.plist.in" > "$A/Info.plist"
install -m 755 "$here/DUHACH" "$A/MacOS/DUHACH"
cp "$here/guide.applescript" "$A/Resources/"
python3 "$here/make-icns.py" "$p/duhach/data/icons/hicolor/scalable/apps/com.mancorporation.Duhach.svg" "$A/Resources/duhach.icns"
"$p/dev-tools/apple/make-usb.sh" "$A/Resources" > "$W/usb.log"
grep -q "package verified" "$W/usb.log" || { cat "$W/usb.log"; exit 1; }
rm -f "$A/Resources/MAN-Mac/README.txt"
chmod 755 "$A/Resources/MAN-Mac/install-man.sh"
if [ -n "${1:-}" ]; then
  rm -rf "$1/DUHACH!.app"; cp -a "$W/DUHACH!.app" "$1/"
  echo "DUHACH!.app -> $1"; exit 0
fi
out="$p/release/DUHACH!-$VER-mac.zip"; rm -f "$out"
# zip with Unix modes (the program must stay executable).
python3 -I - "$W" "$out" <<'PY'
import os, sys, zipfile
w, out = sys.argv[1], sys.argv[2]
with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
    for root, dirs, files in os.walk(w):
        dirs.sort()
        for name in sorted(dirs) + sorted(files):
            path = os.path.join(root, name)
            rel = os.path.relpath(path, w)
            if rel == "usb.log":
                continue
            info = zipfile.ZipInfo(rel + ("/" if os.path.isdir(path) else ""))
            st = os.stat(path)
            info.external_attr = (st.st_mode & 0xFFFF) << 16
            info.create_system = 3
            info.date_time = __import__("time").localtime(st.st_mtime)[:6]
            if os.path.isdir(path):
                z.writestr(info, b"")
            else:
                info.compress_type = zipfile.ZIP_DEFLATED
                z.writestr(info, open(path, "rb").read())
PY
(cd "$p/release" && sha256sum "DUHACH!-$VER-mac.zip" > "DUHACH!-$VER-mac.zip.sha256")
ls -la "$out"
