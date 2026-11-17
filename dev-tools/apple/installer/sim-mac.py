#!/usr/bin/env python3
"""Simulated Mac for MAN's installer menu (no Mac needed): runs the real main.py and
osinstall.py of MAN's Asahi installer copy against a fake diskutil/macOS and prints
every disk change.
  sim-mac.py TREE STICKDIR "answer|answer|..." [fresh|after1]
TREE: unpacked installer-*.tar.gz with installer_data.json copied in (from STICKDIR).
STICKDIR: a MAN-Mac folder (make-usb.sh). after1: the MAN boot loader is installed."""
import sys, os, builtins
tree, stick, answers = sys.argv[1], sys.argv[2], sys.argv[3].split("|")
scen = sys.argv[4] if len(sys.argv) > 4 else "fresh"
sys.path.insert(0, tree); os.chdir(tree)
os.environ.update(MAN_AUTO="1", DISTRO="MAN", REPO_BASE=stick)
import util, diskutil, system, osenum, stub, bugs, osinstall, main as M
from diskutil import Partition
GB = 1000**3
DISK = 1000 * GB


class OS:
    def __init__(s, **k):
        s.__dict__.update(dict(version=None, vgid=None, rec_vgid=None, stub=False, m1n1_ver=None,
                               label="Macintosh HD", sys_volume="disk3s1", admin_users=["man"],
                               attached_partitions=[], bp=None, sys_vol_bootable=True), **k)

    def __str__(s):
        return s.label


macos = OS(version="15.1", vgid="MACOS")
loader = OS(version="12.3", vgid="STUB1", stub=True, m1n1_ver="1.4", label="MAN")
# disk layout: [name, offset, size, type, free space inside an APFS container]
state = {"parts": [["disk0s1", 24576, 500 * 1024**2, "Apple_APFS_ISC", None],
                   ["disk0s2", 600 * 1024**2, 994 * GB, "Apple_APFS", 600 * GB],
                   ["disk0s3", DISK - 5 * GB, 5 * GB, "Apple_APFS_Recovery", None]], "n": 4}
if scen == "after1":
    state["parts"][1][2] = 994 * GB - 3100 * 1024**2
    o = state["parts"][1][1] + state["parts"][1][2]
    state["parts"][2:2] = [["disk0s4", o, 2500 * 1024**2, "Apple_APFS", 2 * GB],
                           ["disk0s5", o + 2500 * 1024**2, 500 * 1024**2, "EFI", None]]
    state["n"] = 6


def log(*a):
    print("   [diskutil]", *a)


class FakeDU:
    FREE_THRESHOLD = 16 * 1024 * 1024

    def __init__(s):
        s.disk_parts = {"disk0": {"Size": DISK, "Content": "GUID_partition_scheme"}}

    def get_info(s): pass
    def get_list(s): pass
    def find_system_disk(s): return "disk0"
    def get_disk_size(s, d): return DISK
    def refresh_part(s, p): pass

    def get_partitions(s, d):
        out, p, prev = [], 40 * 512, "disk0"
        for n, off, size, t, cf in sorted(state["parts"], key=lambda x: x[1]):
            if off - p > s.FREE_THRESHOLD:
                out.append(Partition(name=prev, offset=p, size=off - p, free=True, type=None))
            c = {"CapacityCeiling": size, "CapacityFree": cf, "Volumes": [1]} if t == "Apple_APFS" else None
            out.append(Partition(name=n, offset=off, size=size, free=False, type=t, container=c))
            prev, p = n, off + size
        if DISK - p > s.FREE_THRESHOLD:
            out.append(Partition(name=prev, offset=p, size=DISK - p, free=True, type=None))
        return out

    def get_resize_limits(s, n):
        e = [x for x in state["parts"] if x[0] == n][0]
        return {"MinimumSizePreferred": e[2] - e[4] + 20 * GB}

    def resizeContainer(s, n, size):
        e = [x for x in state["parts"] if x[0] == n][0]
        assert int(size) <= e[2], "grow?"
        e[4] -= e[2] - int(size); e[2] = int(size)
        assert e[4] >= M.MIN_FREE_OS, "macOS left with too little free space"
        log("apfs resizeContainer", n, util.ssize(int(size)))

    def addPartition(s, after, fs, label, size):
        size = int(size)
        parts = s.get_partitions("disk0")
        i = [k for k, p in enumerate(parts) if p.name == after and not p.free][0]
        fr = parts[i + 1]
        assert fr.free, f"no free space after {after}"
        assert size <= fr.size, f"{size} > free {fr.size}"
        n = f"disk0s{state['n']}"; state["n"] += 1
        t = {"apfs": "Apple_APFS", "%EFI%": "EFI", "%Linux%": "Linux"}[fs]
        state["parts"].append([n, fr.offset, size, t, size if t == "Apple_APFS" else None])
        log("addPartition", after, fs, label, util.ssize(size), "->", n)
        return [p for p in s.get_partitions("disk0") if p.name == n][0]


class FakeEnum:
    def __init__(s, *a): pass

    def collect(s, parts):
        for p in parts:
            if p.name == "disk0s2":
                p.os = [macos]
            if p.name == "disk0s4" and scen == "after1":
                p.os = [loader]


class Sys:
    boot_mode = "macOS"; login_user = "man"; chip_id = 0x6000; device_class = "j314sap"
    macos_ver = "15.1"; sys_firmware = "iBoot-11881.41.5"; sfr_full_ver = "24.1.0.0.0,0"
    boot_uuid = "MACOS"; boot_vgid = "MACOS"; default_boot = "MACOS"
    def show(s): pass


class Stub:
    def __init__(s, *a): pass
    def load_ipsw(s, ipsw): print("   [stub] IPSW", ipsw.version)


diskutil.DiskUtil = FakeDU; osenum.OSEnum = FakeEnum; system.SystemInfo = Sys
bugs.run_checks = lambda s: None
stub.StubInstaller = Stub
osinstall.subprocess = type("SP", (), {"run": staticmethod(lambda a, **k: print("   [macOS]", *a)), "PIPE": None})


def do_install(self, total_size=None):
    self.osins.partition_disk(self.part.name, total_size)
    print("   [install] boot loader files + firmware + step2 (shutdown)")


M.InstallerMain.do_install = do_install
M.os.system = lambda c: print("   [system]", c)
M.time.sleep = lambda s: None
it = iter(answers)


def ask(prompt=""):
    a = next(it)
    print(prompt + a)
    return a


builtins.input = ask; util.input_prompt = ask; M.input_prompt = ask
M.InstallerMain(open(os.path.join(tree, "version.tag")).read().strip()).main()
print("FINAL LAYOUT:")
for n, off, size, t, cf in sorted(state["parts"], key=lambda x: x[1]):
    extra = f"  (free in it {util.ssize(cf)})" if t == "Apple_APFS" and cf is not None else ""
    print(f"  {n:8} {t:20} {util.ssize(size):>10}{extra}")
