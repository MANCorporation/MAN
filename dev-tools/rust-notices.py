#!/usr/bin/env python3
"""Write THIRD-PARTY-RUST.txt for a Cargo project: every crate compiled into
the shipped binaries (normal dependencies only, for one target), its license
and the license texts it ships, grouped so each distinct text appears once.

    dev-tools/rust-notices.py PROJECT_DIR TARGET_TRIPLE
    e.g. dev-tools/rust-notices.py man-installer x86_64-unknown-linux-gnu
         dev-tools/rust-notices.py duhach x86_64-pc-windows-gnu
"""
import hashlib, json, os, subprocess, sys

project, target = sys.argv[1], sys.argv[2]
meta = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--format-version", "1", "--filter-platform", target],
    cwd=project))
pkgs = {p["id"]: p for p in meta["packages"]}
nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
members = set(meta["workspace_members"])

# Walk normal (non-dev, non-build) dependencies from the workspace members.
seen, todo = set(), list(members)
while todo:
    for dep in nodes[todo.pop()]["deps"]:
        if any(k["kind"] is None for k in dep["dep_kinds"]) and dep["pkg"] not in seen:
            seen.add(dep["pkg"])
            todo.append(dep["pkg"])
crates = sorted((pkgs[i] for i in seen - members), key=lambda p: (p["name"], p["version"]))

texts = {}   # hash -> (text, [crate names])
rows = []
for p in crates:
    d = os.path.dirname(p["manifest_path"])
    files = sorted(f for f in os.listdir(d)
                   if f.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE", "COPYRIGHT", "UNLICENSE")))
    if p.get("license_file"):
        files.append(p["license_file"])
    for f in dict.fromkeys(files):
        path = os.path.join(d, f)
        if os.path.isfile(path):
            t = open(path, encoding="utf-8", errors="replace").read().strip()
            h = hashlib.sha256(t.encode()).hexdigest()
            texts.setdefault(h, (t, []))[1].append(f'{p["name"]} {p["version"]}')
    rows.append(f'{p["name"]} {p["version"]} — {p.get("license") or "see license file"}'
                + (f' — {p["repository"]}' if p.get("repository") else ""))

name = os.path.basename(os.path.abspath(project))
out = [f"Third-party Rust libraries in {name} ({target})",
       "=" * 60, "",
       f"{name} itself is proprietary (see LICENSE). It is built with the",
       "following open-source Rust libraries, used under their licenses.",
       "Where a library offers a choice (\"MIT OR Apache-2.0\"), MAN uses it",
       "under the first license listed. GTK, GLib, libadwaita and the other C",
       "libraries are not part of these binaries: they are linked dynamically",
       "and keep their own licenses (LGPL-2.1-or-later and others).", "",
       f"{len(rows)} libraries:", ""] + rows + ["", "", "License texts", "=" * 60]
for t, users in sorted(texts.values(), key=lambda v: v[1][0]):
    out += ["", "-" * 60, "Used by: " + ", ".join(users), "-" * 60, "", t]
open(os.path.join(project, "THIRD-PARTY-RUST.txt"), "w", encoding="utf-8").write("\n".join(out) + "\n")
print(f"{project}/THIRD-PARTY-RUST.txt: {len(rows)} crates, {len(texts)} license texts")
