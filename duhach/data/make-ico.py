#!/usr/bin/env python3
"""DUHACH!'s Windows icon: SVG -> .ico with PNG entries (16..256 px), rendered
by tools/rsvg/rsvg-convert. No Windows tools needed.
    make-ico.py icon.svg out.ico"""
import os, struct, subprocess, sys, tempfile
svg, out = sys.argv[1], sys.argv[2]
rsvg = os.path.expanduser("~/Documents/MANRUST/tools/rsvg/rsvg-convert")
sizes = [16, 24, 32, 48, 64, 128, 256]
pngs = []
with tempfile.TemporaryDirectory() as t:
    for s in sizes:
        f = os.path.join(t, f"{s}.png")
        subprocess.run([rsvg, "-w", str(s), "-h", str(s), svg, "-o", f], check=True)
        pngs.append(open(f, "rb").read())
head = struct.pack("<HHH", 0, 1, len(sizes))
offset = 6 + 16 * len(sizes)
dirs = b""
for s, data in zip(sizes, pngs):
    dirs += struct.pack("<BBBBHHII", s % 256, s % 256, 0, 0, 1, 32, len(data), offset)
    offset += len(data)
open(out, "wb").write(head + dirs + b"".join(pngs))
