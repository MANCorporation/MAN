# dev-tools (saved 2026-10-18)

Tools Claude used to build and test MAN without root on the host. They were
written for a /tmp scratch folder, so paths inside some scripts still point
there: copy `vm/` back to a scratch folder (or adjust `S=`) before use.

- `repo/` — local apt repo installed into the live image by `repo/live-prep.sh`
  (run *inside* a VM as root, repo shared over virtio-9p): man-installer and
  man-system-tools 3.1.0, cryptsetup, e2fsprogs, hostname, the 7.3-rc7 kernel
  and modules. Refresh `Packages` with `apt-ftparchive packages . > Packages`.
- `repo/lang/` + `repo/lang-prep.sh` (MAN 10.5) — locales/libc-l10n 2.43-6
  (must match the image's libc; from snapshot.debian.org), Noto fonts, and the
  script that enables the 74 MAN languages and runs locale-gen.
- `vm/uefi.sh`, `vm/boot.sh` — headless QEMU (UEFI / direct kernel) with
  serial, monitor and QMP sockets; `ser.py` talks to the root debug shell
  (`systemd.debug_shell=ttyS1`), `mon.py`/`qmp.py`/`keys.sh` drive the screen,
  `ppm2png.py`/`sheet.py` turn screendumps into PNGs.
- `vm/update-live.sh`, `fresh-install.sh`, `oobe-walk.sh` — rebuild packages
  into the live image, install to a blank disk, walk MAN Setup.

Building the Rust GUI needs GTK 4 / libadwaita dev files. Easiest:
`sudo apt install libgtk-4-dev libadwaita-1-dev` (Claude had no sudo and used
an unpacked sysroot via `vm/buildenv.sh`, which is gone after a reboot).

Keep growing VM disks out of /tmp (it is RAM); use a folder on /home.
