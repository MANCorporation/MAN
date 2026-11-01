#!/usr/bin/env python3
"""Download GTK 4 + libadwaita for Windows (MSYS2) with all runtime
dependencies and unpack them into ./root (used to build and ship DUHACH!).

    python3 fetch-gtk.py              # x64: ucrt64, needs ucrt64.db unpacked in ./db
    python3 fetch-gtk.py clangarm64   # ARM64: needs clangarm64.db unpacked in ./db-clangarm64
"""
import os, re, subprocess, sys, urllib.request

# Works in tools/gtk-win (the default) or wherever GTK_WIN points.
HERE = os.environ.get("GTK_WIN") or os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "tools", "gtk-win")
ENV = sys.argv[1] if len(sys.argv) > 1 else "ucrt64"
REPO = f"https://repo.msys2.org/mingw/{ENV}/"
WANT = ["gtk4", "libadwaita", "adwaita-icon-theme", "hicolor-icon-theme", "librsvg", "gdk-pixbuf2"]
PREFIX = {"ucrt64": "mingw-w64-ucrt-x86_64-", "clangarm64": "mingw-w64-clang-aarch64-"}[ENV]
DB = os.path.join(HERE, "db" if ENV == "ucrt64" else f"db-{ENV}")
if ENV == "clangarm64":
    # Linking with clang/lld on Linux needs the mingw-w64 CRT and runtimes.
    WANT += ["crt-git", "headers-git", "libunwind", "libc++", "compiler-rt", "winpthreads"]
# Build-only or huge packages DUHACH! never needs at run time.
SKIP = {"python", "tcl", "tk", "gcc-libs-dummy"}


def parse(desc):
    fields, key = {}, None
    for line in open(desc, encoding="utf-8"):
        line = line.rstrip("\n")
        if line.startswith("%") and line.endswith("%"):
            key = line.strip("%")
            fields[key] = []
        elif line and key:
            fields[key].append(line)
    return fields


pkgs, provides = {}, {}
for d in os.listdir(DB):
    f = parse(os.path.join(DB, d, "desc"))
    name = f["NAME"][0]
    pkgs[name] = f
    for p in f.get("PROVIDES", []):
        provides[re.split(r"[<>=]", p)[0]] = name


def resolve(dep):
    dep = re.split(r"[<>=]", dep)[0]
    return dep if dep in pkgs else provides.get(dep)


todo, seen = [PREFIX + w for w in WANT], set()
while todo:
    n = resolve(todo.pop())
    if not n or n in seen or n.removeprefix(PREFIX) in SKIP:
        continue
    seen.add(n)
    todo += pkgs[n].get("DEPENDS", [])

os.makedirs(os.path.join(HERE, "pkgs"), exist_ok=True)
os.makedirs(os.path.join(HERE, "root"), exist_ok=True)
for n in sorted(seen):
    fn = pkgs[n]["FILENAME"][0]
    path = os.path.join(HERE, "pkgs", fn)
    if not os.path.exists(path):
        print("get", fn, flush=True)
        urllib.request.urlretrieve(REPO + fn, path)
    subprocess.run(["tar", "-xf", path, "-C", os.path.join(HERE, "root"), "--exclude=.BUILDINFO",
                    "--exclude=.MTREE", "--exclude=.PKGINFO"], check=True)
print(len(seen), "packages:", " ".join(sorted(x.removeprefix(PREFIX) for x in seen)))
