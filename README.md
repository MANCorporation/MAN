# MAN

MAN is a GNOME 51 desktop operating system with its own installer, first-start
setup, Btrfs snapshots, signed system updates and DUHACH! (installs MAN from
Windows).

| Folder | What |
|---|---|
| `man-installer/` | MAN Installer, MAN Setup, Snapshots, MAN Update, man-mkupd (Rust, GTK 4) |
| `duhach/` | DUHACH! for Windows |
| `gnome51-fix/`, `iso/`, `make-*.sh`, `run-gnome.sh` | building and testing the system image and ISO |
| `src/` | MAN's patches to GNOME Settings and GNOME Web (GPL, see below) |
| `dev-tools/` | test harness, translation sources |

Docs: [ISO.md](ISO.md), [UPDATES.md](UPDATES.md), `man-installer/README.md`,
`duhach/README.md`.

## License

MAN's own code, artwork and documentation are **proprietary — Copyright (c)
2026 MAN Corporation, all rights reserved** ([LICENSE](LICENSE)).
Third-party software keeps its own license: MAN's patches in `src/` are
under the GPL of the program they change, and the Rust libraries, the
operating system components and the libraries bundled with DUHACH! are
listed in [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).
