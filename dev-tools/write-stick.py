#!/usr/bin/env python3
"""Write an image to a USB stick through UDisks (asks for the password in a window),
check it by reading it back, switch the stick off.
  write-stick.py IMAGE /dev/sdX write|verify|off [UDISKS_DRIVE_ID]"""
import hashlib, os, sys
from gi.repository import Gio, GLib
iso, dev, mode = sys.argv[1], sys.argv[2], sys.argv[3]
bus = Gio.bus_get_sync(Gio.BusType.SYSTEM)
obj = "/org/freedesktop/UDisks2/block_devices/" + os.path.basename(dev)
AUTH = Gio.DBusCallFlags.ALLOW_INTERACTIVE_AUTHORIZATION
TIMEOUT = 300000  # the password window waits up to 5 minutes


def open_fd(method):
    res, fdl = bus.call_with_unix_fd_list_sync("org.freedesktop.UDisks2", obj, "org.freedesktop.UDisks2.Block",
                                               method, GLib.Variant("(a{sv})", ({},)), None, AUTH, TIMEOUT, None, None)
    return fdl.get(res.unpack()[0])


size = os.path.getsize(iso)
if mode == "write":
    with open(iso, "rb") as f, os.fdopen(open_fd("OpenForRestore"), "wb", buffering=0) as o:
        done = 0
        while (b := f.read(4 << 20)):
            o.write(b); done += len(b)
            if done % (256 << 20) < (4 << 20):
                print(f"{done * 100 // size}%", flush=True)
        os.fsync(o.fileno())
    print("written")
else:
    h, left = hashlib.sha256(), size
    with os.fdopen(open_fd("OpenForBackup"), "rb") as i:
        while left:
            b = i.read(min(4 << 20, left)); h.update(b); left -= len(b)
    print("stick", h.hexdigest())
    if mode == "off":
        bus.call_sync("org.freedesktop.UDisks2", "/org/freedesktop/UDisks2/drives/" + sys.argv[4],
                      "org.freedesktop.UDisks2.Drive", "PowerOff", GLib.Variant("(a{sv})", ({},)), None, AUTH, TIMEOUT, None)
