# MAN for ARM64

The x86-64 image grew by hand; the ARM64 one is made from Debian sid arm64 (which has
GNOME 51) in an emulated ARM64 machine. Everything lives in `lfs/arm64/`.

1. `tools/qemu-arm`: Debian qemu-system-arm + qemu-efi-aarch64, unpacked.
   `lfs/arm64/debian-13-nocloud-arm64.qcow2`: Debian's image; `builder.sh` starts it.
   vda = builder, vdb = target image, 9p `lfs/arm64/share` = `/mnt/share`.
   Root shell on the serial port (debug-shell); `vmrun.py SOCK TIMEOUT CMD` runs commands.
2. Kernel 7.3-rc7: arm64 defconfig + the options of the x86-64 MAN kernel, built
   on the PC with `make ARCH=arm64 LLVM=1` → `lfs/arm64/kernel`.
3. `guest/` (copied to share): `check-pkgs.sh` + `mkroot.sh` (MAN's package list on
   sid arm64), `mksysroot.sh` (sysroot for `cross-env.sh`), `mkbuildenv.sh` +
   `build-apps.sh` (patched Settings, PCD), `finish.sh` (kernel, MAN packages,
   MAN's files, accounts, version, initramfs).
4. MAN apps: `. dev-tools/arm64/cross-env.sh; DEB_ARCH=arm64 man-installer/packaging/build-debs.sh`.
5. `./make-iso.sh lfs/arm64/man-gnome-arm64.img` → `release/MAN-<version>-arm64.iso`.
   `run.sh image|iso` boots them, with no window (emulated, so slow).

`man-live-config/` is that package's tree (1.0.5): no x86 kernel dependency, and its
initramfs hook finds libraries for any processor.
