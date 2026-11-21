#!/bin/bash
# DUHACH! for Mac: release/DUHACH-<ver>-mac.pkg (installer package: the app in
# Applications + DUHACH! in the menu bar at each login, see make-pkg.py) and
# release/DUHACH!-<ver>-mac.zip (just DUHACH!.app).
# The app = this folder's guide (AppleScript, run by osascript: no compiling, no Mac
# needed to build) + MAN-Mac (MAN's copy of the Asahi installer and MAN's boot loader
# package, from dev-tools/apple/make-usb.sh). make-iso.sh also puts the app on the
# MAN_MAC partition of the Mac live USB stick.
#   build-mac.sh [OUTDIR]   (default: release/, zip; with OUTDIR: just DUHACH!.app there)
set -euo pipefail
here=$(cd -- "$(dirname -- "$(readlink -f "$0")")" && pwd)
p=$(cd -- "$here/../.." && pwd)
VER=1.1.0
W=$(mktemp -d); trap 'rm -rf "$W"' EXIT
A="$W/DUHACH!.app/Contents"
mkdir -p "$A/MacOS" "$A/Resources"
sed "s/@VERSION@/$VER/" "$here/Info.plist.in" > "$A/Info.plist"
install -m 755 "$here/DUHACH" "$A/MacOS/DUHACH"
cp "$here/guide.applescript" "$here/main.js" "$here/duhach-core.js" "$A/Resources/"
# tools.html (Restore MAN Snapshot, More Settings) uses the installer page's styles.
python3 - "$here/ui/index.html" "$here/ui/tools.html" "$A/Resources/tools.html" <<'PY'
import re, sys
style = re.search(r"<style>(.*?)</style>", open(sys.argv[1], encoding="utf-8").read(), re.S).group(1)
html = open(sys.argv[2], encoding="utf-8").read()
assert "/*STYLE*/" in html
open(sys.argv[3], "w", encoding="utf-8").write(html.replace("/*STYLE*/", style))
PY
# The menu bar icon: a template image (black + transparency; macOS colours it).
"$p/tools/rsvg/rsvg-convert" -w 36 -h 36 "$p/duhach/data/icons/hicolor/symbolic/apps/com.mancorporation.Duhach-symbolic.svg" -o "$A/Resources/menubar.png"
svg="$p/duhach/data/icons/hicolor/scalable/apps/com.mancorporation.Duhach.svg"
python3 "$here/make-icns.py" "$svg" "$A/Resources/duhach.icns"
# ui.html: the page with the app icon inlined (the web view loads it as one string).
python3 - "$svg" "$here/ui/index.html" "$A/Resources/ui.html" <<'PY'
import base64, sys, gi
gi.require_version("GdkPixbuf", "2.0")
from gi.repository import GdkPixbuf
ok, png = GdkPixbuf.Pixbuf.new_from_file_at_size(sys.argv[1], 256, 256).save_to_bufferv("png", [], [])
html = open(sys.argv[2], encoding="utf-8").read()
assert '"/*ICON*/"' in html and "/*DATA*/null" in html
html = html.replace('"/*ICON*/"', '"data:image/png;base64,' + base64.b64encode(png).decode() + '"')
open(sys.argv[3], "w", encoding="utf-8").write(html)
PY
"$p/dev-tools/apple/make-usb.sh" "$A/Resources" > "$W/usb.log"
grep -q "package verified" "$W/usb.log" || { cat "$W/usb.log"; exit 1; }
rm -f "$A/Resources/MAN-Mac/README.txt"
chmod 755 "$A/Resources/MAN-Mac/install-man.sh"
if [ -n "${1:-}" ]; then
  rm -rf "$1/DUHACH!.app"; cp -a "$W/DUHACH!.app" "$1/"
  echo "DUHACH!.app -> $1"; exit 0
fi
node "$here/test-core.js"
python3 "$here/make-pkg.py" "$W/DUHACH!.app" "$VER" "$p/release/DUHACH-$VER-mac.pkg"
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
