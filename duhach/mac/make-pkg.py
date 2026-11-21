#!/usr/bin/env python3
"""make-pkg.py APP VERSION OUT.pkg — DUHACH! for Mac as a macOS installer package,
made on Linux (no pkgbuild/productbuild/xar needed).

The package is a "payload-free" product archive (like pkgbuild --nopayload): its
postinstall script, which runs as root, copies DUHACH!.app (packed next to it in the
Scripts archive) to /Applications, installs the LaunchAgent that starts DUHACH! in
the menu bar at each login, and starts it for the user who is logged in.

Format: xar archive (header, zlib-compressed XML table of contents, heap) holding
Distribution and duhach.pkg/{PackageInfo, Scripts}; Scripts = gzip of a cpio
(odc) archive. The package is not signed: macOS asks once (System Settings →
Privacy & Security → Open Anyway), like for the zip of DUHACH! 1.0.
"""
import gzip
import hashlib
import io
import os
import stat
import struct
import sys
import time
import zlib
from xml.sax.saxutils import escape

APP, VERSION, OUT = sys.argv[1], sys.argv[2], sys.argv[3]
IDENT = "com.mancorporation.duhach"
LABEL = "com.mancorporation.Duhach"
APPNAME = os.path.basename(APP.rstrip("/"))  # DUHACH!.app

PREINSTALL = """#!/bin/sh
# DUHACH! for Mac: a running DUHACH! is stopped before the new one is copied.
pkill -f 'Resources/main[.]js' 2>/dev/null || true
exit 0
"""

POSTINSTALL = f"""#!/bin/sh
# DUHACH! for Mac {VERSION}: install the app, start it at each login, start it now.
set -e
cd "$(dirname "$0")"
APP="/Applications/{APPNAME}"
rm -rf "$APP"
cp -R "./{APPNAME}" /Applications/
chown -R root:wheel "$APP"
chmod -R go-w "$APP"
xattr -dr com.apple.quarantine "$APP" 2>/dev/null || true

AGENT=/Library/LaunchAgents/{LABEL}.plist
cat > "$AGENT" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Label</key><string>{LABEL}</string>
	<key>ProgramArguments</key>
	<array><string>$APP/Contents/MacOS/DUHACH</string><string>--menubar</string></array>
	<key>RunAtLoad</key><true/>
	<key>ProcessType</key><string>Interactive</string>
	<key>LimitLoadToSessionType</key><string>Aqua</string>
</dict>
</plist>
EOF
chown root:wheel "$AGENT"
chmod 644 "$AGENT"

# The user at the screen: DUHACH! in the menu bar now, and the installer window.
user=$(stat -f %Su /dev/console 2>/dev/null || true)
if [ -n "$user" ] && [ "$user" != root ]; then
	uid=$(id -u "$user")
	launchctl bootout "gui/$uid/{LABEL}" 2>/dev/null || true
	launchctl bootstrap "gui/$uid" "$AGENT" 2>/dev/null || launchctl asuser "$uid" launchctl load -w "$AGENT" 2>/dev/null || true
	launchctl asuser "$uid" sudo -u "$user" open "$APP" 2>/dev/null || true
fi
exit 0
"""


# ── cpio (odc) ──
def cpio_odc(entries):
    """entries: (name, mode, data or None for directories), names like ./a/b"""
    out = io.BytesIO()
    ino = 1
    now = int(time.time())

    def header(name, mode, size, nlink):
        nonlocal ino
        n = name.encode() + b"\0"
        h = "070707" + "%06o" % 0 + "%06o" % ino + "%06o" % mode + "%06o" % 0 + "%06o" % 0 + \
            "%06o" % nlink + "%06o" % 0 + "%011o" % now + "%06o" % len(n) + "%011o" % size
        ino += 1
        out.write(h.encode() + n)

    for name, mode, data in entries:
        if data is None:
            header(name, mode, 0, 2)
        else:
            header(name, mode, len(data), 1)
            out.write(data)
    header("TRAILER!!!", 0, 0, 1)
    return out.getvalue()


def app_entries():
    root = os.path.dirname(os.path.abspath(APP))
    entries = []
    for dirpath, dirs, files in os.walk(APP):
        dirs.sort()
        rel = os.path.relpath(dirpath, root)
        entries.append(("./" + rel, stat.S_IFDIR | 0o755, None))
        for f in sorted(files):
            path = os.path.join(dirpath, f)
            if os.path.islink(path):
                raise SystemExit(f"symbolic links are not supported: {path}")
            mode = 0o755 if os.access(path, os.X_OK) else 0o644
            entries.append(("./" + os.path.relpath(path, root), stat.S_IFREG | mode, open(path, "rb").read()))
    return entries


scripts = cpio_odc([(".", stat.S_IFDIR | 0o755, None),
                    ("./preinstall", stat.S_IFREG | 0o755, PREINSTALL.encode()),
                    ("./postinstall", stat.S_IFREG | 0o755, POSTINSTALL.encode())] + app_entries())
scripts_gz = gzip.compress(scripts, 9, mtime=0)
kbytes = len(scripts) // 1024

package_info = f"""<?xml version="1.0" encoding="utf-8"?>
<pkg-info overwrite-permissions="true" relocatable="false" identifier="{IDENT}" postinstall-action="none" version="{VERSION}" format-version="2" auth="root">
    <scripts>
        <preinstall file="./preinstall"/>
        <postinstall file="./postinstall"/>
    </scripts>
</pkg-info>
""".encode()

distribution = f"""<?xml version="1.0" encoding="utf-8"?>
<installer-gui-script minSpecVersion="2">
    <title>DUHACH! {VERSION}</title>
    <options customize="never" require-scripts="true" hostArchitectures="arm64,x86_64"/>
    <domains enable_anywhere="false" enable_currentUserHome="false" enable_localSystem="true"/>
    <volume-check><allowed-os-versions><os-version min="13.5"/></allowed-os-versions></volume-check>
    <welcome mime-type="text/plain"><![CDATA[DUHACH! puts MAN on this Mac, next to macOS.

It installs the DUHACH! app in Applications and keeps DUHACH! in the menu bar, with Boot into MAN, Restore MAN Snapshot and More Settings. After installing, DUHACH! opens and guides you.]]></welcome>
    <choices-outline>
        <line choice="default">
            <line choice="{IDENT}"/>
        </line>
    </choices-outline>
    <choice id="default"/>
    <choice id="{IDENT}" visible="false">
        <pkg-ref id="{IDENT}"/>
    </choice>
    <pkg-ref id="{IDENT}" version="{VERSION}" installKBytes="{kbytes}" onConclusion="none">#duhach.pkg</pkg-ref>
</installer-gui-script>
""".encode()


# ── xar ──
def xar(tree):
    """tree: list of (name, bytes) or (name, [children]) — written to OUT."""
    heap = io.BytesIO()
    heap.write(b"\0" * 20)  # the TOC checksum (SHA-1), filled in below
    ids = [0]

    def entry(name, content):
        ids[0] += 1
        fid = ids[0]
        mtime = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
        if isinstance(content, list):
            inner = "".join(entry(n, c) for n, c in content)
            return (f'<file id="{fid}"><name>{escape(name)}</name><type>directory</type>'
                    f'<mode>0755</mode><uid>0</uid><gid>0</gid><mtime>{mtime}</mtime>{inner}</file>')
        off = heap.tell()
        heap.write(content)
        sha = hashlib.sha1(content).hexdigest()
        return (f'<file id="{fid}"><name>{escape(name)}</name><type>file</type>'
                f'<data><length>{len(content)}</length><offset>{off}</offset><size>{len(content)}</size>'
                f'<encoding style="application/octet-stream"/>'
                f'<extracted-checksum style="sha1">{sha}</extracted-checksum>'
                f'<archived-checksum style="sha1">{sha}</archived-checksum></data>'
                f'<mode>0644</mode><uid>0</uid><gid>0</gid><mtime>{mtime}</mtime></file>')

    files = "".join(entry(n, c) for n, c in tree)
    toc = ('<?xml version="1.0" encoding="UTF-8"?>\n<xar><toc>'
           '<checksum style="sha1"><offset>0</offset><size>20</size></checksum>'
           f'<creation-time>{time.strftime("%Y-%m-%dT%H:%M:%S")}</creation-time>{files}</toc></xar>\n').encode()
    ztoc = zlib.compress(toc, 9)
    data = heap.getvalue()
    data = hashlib.sha1(ztoc).digest() + data[20:]
    with open(OUT, "wb") as f:
        f.write(struct.pack(">IHHQQI", 0x78617221, 28, 1, len(ztoc), len(toc), 1))
        f.write(ztoc)
        f.write(data)


xar([("Distribution", distribution),
     ("duhach.pkg", [("PackageInfo", package_info), ("Scripts", scripts_gz)])])
print(f"{OUT}: {os.path.getsize(OUT) / 1e6:.1f} MB")
