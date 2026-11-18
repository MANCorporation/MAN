#!/usr/bin/env python3
"""MAN: note the macOS version on the EFI system partition (man/macos-version),
so MAN's boot menu can name it ("macOS Golden Gate") and show its icon.
Linux cannot read APFS, so this is written while the installer runs in macOS.
    patch-macos-version.py INSTALLER_TREE
"""
import sys, os
p = os.path.join(sys.argv[1], "osinstall.py")
s = open(p).read()
old = '''                shutil.copytree(self.firmware_package.path, base)
'''
new = '''                shutil.copytree(self.firmware_package.path, base)
                # MAN: the macOS version for MAN's boot menu (best effort).
                try:
                    import plistlib
                    with open("/System/Library/CoreServices/SystemVersion.plist", "rb") as vf:
                        ver = plistlib.load(vf).get("ProductVersion", "")
                    os.makedirs(os.path.join(mountpoint, "man"), exist_ok=True)
                    with open(os.path.join(mountpoint, "man", "macos-version"), "w") as vf:
                        vf.write(ver + "\\n")
                except Exception as e:
                    logging.info(f"MAN: macOS version not noted: {e}")
'''
if "MAN: the macOS version" in s:
    sys.exit(0)
assert s.count(old) == 1, "osinstall.py changed"
open(p, "w").write(s.replace(old, new))
