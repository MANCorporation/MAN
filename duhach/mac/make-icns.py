#!/usr/bin/env python3
"""SVG -> .icns (PNG entries 128..1024) with GdkPixbuf, no macOS tools.
   make-icns.py icon.svg out.icns"""
import struct, sys
import gi
gi.require_version("GdkPixbuf", "2.0")
from gi.repository import GdkPixbuf
svg, out = sys.argv[1], sys.argv[2]
entries = b""
for kind, size in ((b"ic07", 128), (b"ic08", 256), (b"ic09", 512), (b"ic10", 1024)):
    pb = GdkPixbuf.Pixbuf.new_from_file_at_size(svg, size, size)
    ok, png = pb.save_to_bufferv("png", [], [])
    entries += kind + struct.pack(">I", len(png) + 8) + png
open(out, "wb").write(b"icns" + struct.pack(">I", len(entries) + 8) + entries)
