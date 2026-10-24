# MAN-Rust Distribution Build Plan

## Overview
Building a custom Linux distribution called **MAN** (MAN-Rust Edition) following the
Linux From Scratch (LFS) 12.4 guide with maximum Rust usage, kernel 7.3, targeting
x86_64 (with future aarch64 support).

## Environment
- **Host**: Debian 13 (trixie), x86_64
- **CPU**: 24 cores
- **RAM**: 94 GB
- **Disk**: 167 GB available at /home/kristihack/Documents/MANRUST
- **Rust**: 1.99.0 (installed via rustup)
- **C Compiler**: gcc 14.2.0 (host)
- **LFS Directory**: /home/kristihack/Documents/MANRUST/lfs
- **Kernel**: Linux 7.3-rc7 (latest 7.3 release from git.kernel.org)

## Rust Integration Strategy
Since 100% Rust would break compatibility (LFS build requires a C toolchain), we
follow the LFS guide's structure but maximize Rust where compatible:

### Rust replacements (drop-in or API-compatible):
1. **coreutils** → `uutils/coreutils` (Rust reimplementation, 100% CLI compatible)
2. **grep** → `ripgrep` (rg, compatible CLI flags)
3. **find** → `findutils` replaced by `fd` (optional, keep find for build scripts)
4. **gzip/xz/zstd** → Use Rust `flate2`, `xz`, `zstd` crates in custom tools
5. **file** → Custom Rust tool using `infer` crate
6. **Boot scripts** → Written in Rust (replaces some SysVinit shell scripts)
7. **Init system** → Custom Rust init (compatible with SysVinit interface)
8. **Kernel modules** → Rust kernel modules (Linux 6.1+ Rust support)

### Rust-native additions:
1. `man-rust-init` - Rust-based init system (SysV-compatible)
2. `man-rust-ctl` - Service manager CLI
3. `man-rust-builder` - Build automation tool
4. Rust kernel module for custom hardware

### Compatibility constraints:
- All GNU C programs still built from source (required for glibc, gcc, etc.)
- glibc is mandatory (Rust links against it)
- Shell scripts maintained in POSIX sh
- All standard Unix command-line interfaces preserved

## Build Phases

### Phase 0: Host Preparation ✅
- [x] Install build tools (bison, flex, gperf, texinfo extracted to ~/opt)
- [x] Install Rust toolchain (1.99.0)
- [x] Create LFS directory at $LFS=/home/kristihack/Documents/MANRUST/lfs
- [x] Download all LFS source packages
- [x] Download LFS patches
- [x] Download kernel 7.3-rc7

### Phase 1: Cross-Toolchain (LFS Ch 5)
1. Binutils-2.45 Pass 1 (cross-compiler)
2. GCC-15.2.0 Pass 1 (C, C++ cross-compiler)
3. Linux 7.3 API Headers (instead of 6.16.1)
4. Glibc-2.42
5. Libstdc++ from GCC-15.2.0

### Phase 2: Temporary System (LFS Ch 6)
- Build in chroot using user namespace (unshare --user --map-root-user)
- M4, Ncurses, Bash, Coreutils, Diffutils, File, Findutils, Gawk, Grep, Gzip,
  Make, Patch, Sed, Tar, Xz, Binutils Pass 2, GCC Pass 2
- Gettext, Bison, Perl, Python, Texinfo, Util-linux

### Phase 3: Final System (LFS Ch 8)
- All system packages following LFS
- **Rust additions**: Install rustc/cargo, build uutils/coreutils, ripgrep
- **Rust native tools**: Build MAN-specific utilities

### Phase 4: Kernel (LFS Ch 10) ✅
- [x] Configure kernel 7.3-rc7 with CONFIG_RUST=y, CONFIG_RUST_DEBUG_ASSERTIONS=y
- [x] Install Rust toolchain (rustc 1.99.0) in chroot
- [x] Install bindgen, libclang, libLLVM + all dependencies in chroot
- [x] Install rust-src at $LFS/usr/local/lib/rustlib/src/
- [x] Build kernel with CONFIG_RUST=y (RUSTC, HOSTCC, HOSTRUSTC all work)
- [x] Install kernel to /boot/vmlinuz-7.3-rc7
- [x] Build and install kernel modules (12 modules, depmod succeeded)
- [x] Kernel build script saved at $LFS/sources/04-kernel-build.sh

### Phase 5: System Configuration (LFS Ch 9) ✅
- [x] Create /etc/fstab (ext4 root on virtio-blk, tmpfs for /tmp and /run)
- [x] Create /etc/hosts, /etc/passwd, /etc/group, /etc/nsswitch.conf
- [x] Create /etc/hostname (mandroid), /etc/resolv.conf
- [x] Create boot scripts in /etc/init.d/ (boot, network, syslog, rc)
- [x] Create /etc/inittab (SysVinit compatible)
- [x] Build Rust-based init system (man-init) at /sbin/init
- [x] Install agetty from host for console login
- [x] Install bash binary from LFS tools build
- [x] Create /etc/rc.local (welcome message)

### Phase 6: Boot Test (QEMU) ✅
- [x] Create QEMU disk image (ext4, 8GB) with root filesystem
- [x] Boot with qemu-system-x86_64 + KVM
- [x] Kernel 7.3.0-rc7 boots successfully with Rust support
- [x] ext4 root filesystem mounted from virtio-blk
- [x] Rust init mounts all filesystems, remounts root rw
- [x] rc.local executes ("MAndroid Linux - Welcome!")
- [x] agetty starts on tty1 (login prompt ready)

## Build Order
The build follows the LFS guide exactly for the C toolchain and C programs,
with Rust components added at integration points that don't break compatibility.
