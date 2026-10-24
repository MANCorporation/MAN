# MANRUST - GNOME 51 Integration Status

## COMPLETE: GNOME 51 Desktop Successfully Booted in QEMU

The image was repaired and re-verified on 2026-10-18. See
`GNOME51_REPAIR.md` for the root causes, exact invariants, and remaining package
management caveat. Use `./run-gnome.sh graphical` instead of copying a raw QEMU
command.

### Success Criteria - ALL MET
- **Kernel 7.3-rc7** boots with `CONFIG_RUST=y`
- **ext4 root filesystem** mounts successfully on `/dev/vda`
- **systemd 262** starts and reaches `graphical.target`
- **gnome-shell 51** binary present at `/usr/bin/gnome-shell`
- **gdm3** (GNOME Display Manager) **STARTED OK**
- **graphical.target** **REACHED**
- All essential services running: dbus, systemd-logind, polkit, NetworkManager, accounts-daemon, udisks2

### Boot Command (WORKING - CLI mode)
```bash
export LFS=/home/kristihack/Documents/MANRUST/lfs

qemu-system-x86_64 \
    -m 4096 -smp 4 \
    -kernel $LFS/boot/vmlinuz-7.3-rc7 \
    -append "root=/dev/vda rootwait console=ttyS0,115200 rw loglevel=7" \
    -drive file=$LFS/man-gnome.img,format=raw,if=virtio \
    -nographic -enable-kvm -no-reboot \
    -serial mon:stdio -display none
```

### Boot Command (WORKING - Graphical mode, GUI visible)
```bash
export LFS=/home/kristihack/Documents/MANRUST/lfs

qemu-system-x86_64 \
    -m 4096 -smp 4 \
    -kernel $LFS/boot/vmlinuz-7.3-rc7 \
    -append "root=/dev/vda rootwait console=ttyS0,115200 rw loglevel=7" \
    -drive file=$LFS/man-gnome.img,format=raw,if=virtio \
    -enable-kvm -no-reboot \
    -display gtk,show-cursor=on \
    -vga virtio \
    -device virtio-keyboard-pci \
    -device virtio-mouse-pci \
    -serial stdio
```

### Root filesystem mode
The launcher uses `rw`. The earlier remount and virtual-filesystem failures were
also caused by incorrect ownership of setuid `mount` after rootless package
extraction; that ownership is now corrected in the disk image.

### Service Status Summary
- **FAILED (0)** in the final diagnostic boot.
- GDM's dynamic greeter, its user manager, the Wayland session, and GNOME Shell
  51 all remained running.
- The virtual-filesystem mount units and `systemd-remount-fs` completed
  successfully.

### OS Identity Changes (v18)
- **Hostname**: Changed from `mandroid` → `man`
- **OS name**: Changed from `Debian GNU/Linux forky/sid` → `MAN Linux`
- **rc.local**: Changed "MAndroid Linux" → "MAN Linux"
- **Image**: Renamed from `mandroid-gnome.img` → `man-gnome.img`

### How to Rebuild Individual GNOME App from Source
1. Use BuildStream: `bst build elements/core/gnome-shell.bst` from 
   `/home/kristihack/Documents/MAN/sources/upstream/gnome-build-meta-51/`
2. Or extract `.deb` → modify source → rebuild with `dpkg-buildpackage`
3. Only the modified app rebuilds, not entire GNOME

### GNOME 51 Package Sources
- **2054 .deb files**: `/home/kristihack/Documents/MAN/work/apt-gnome51-amd64/archives/`
- **67 packages at GNOME 51.0**: `gnome-shell_51.0-2_amd64.deb`, `gdm3_51.0-1_amd64.deb`, `mutter_51.0-2_amd64.deb`
- **BuildStream project**: `/home/kristihack/Documents/MAN/sources/upstream/gnome-build-meta-51/` (544 `.bst` files)

### Kernel Configuration (Rust-focused)
- `CONFIG_RUST=y`, `CONFIG_RUST_IS_AVAILABLE=y`
- `CONFIG_DEVTMPFS=y`, `CONFIG_EXT4_FS=y`, `CONFIG_VIRTIO=y`
- Kernel size: 15,418,368 bytes

### Rust Tools Installed in MANRUST
- `rustc` 1.99.0 at `/usr/local/bin/`
- `cargo`, `bindgen`
- `uutils/coreutils` v0.12.0, `ripgrep` v15.2.0, `fd` v10.5.0
