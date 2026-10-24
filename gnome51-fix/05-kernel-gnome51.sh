#!/bin/bash
# MAN kernel 7.3-rc7 for GNOME 51: original Rust config + desktop requirements.
# Run INSIDE the LFS chroot (gcc 15.2, rustc 1.99, bindgen 0.73).
set -eo pipefail
export PATH=/usr/local/bin:/usr/bin:/usr/sbin:/tools/bin
KVER=7.3-rc7; SRC=/sources/linux-$KVER; B=/sources/linux-build-gnome51
rm -rf "$B"; cp -a "$SRC" "$B"; cd "$B"
make mrproper >/dev/null
cp /sources/config-7.3-rc7.rust-base .config
E(){ for o; do ./scripts/config --enable "$o"; done; }
E USER_NS FUSE_FS CUSE BPF_SYSCALL CGROUP_BPF MEMCG PSI OVERLAY_FS \
  SQUASHFS SQUASHFS_XZ SQUASHFS_ZSTD FANOTIFY INPUT_UINPUT HW_RANDOM_VIRTIO \
  SECURITY_LANDLOCK UDMABUF DMABUF_HEAPS \
  BTRFS_FS BTRFS_FS_POSIX_ACL DM_CRYPT CRYPTO_XTS CRYPTO_AES_NI_INTEL CRYPTO_SHA256 CRYPTO_USER_API_SKCIPHER CRYPTO_USER_API_HASH \
  EFIVAR_FS BLK_DEV_NVME SYSFB_SIMPLEFB DRM_SIMPLEDRM FB_EFI EFI_PARTITION MSDOS_PARTITION \
  FB DRM_FBDEV_EMULATION FRAMEBUFFER_CONSOLE FRAMEBUFFER_CONSOLE_DETECT_PRIMARY
make olddefconfig >/dev/null
for o in RUST USER_NS FUSE_FS BTRFS_FS DM_CRYPT CRYPTO_XTS EFIVAR_FS BLK_DEV_NVME FB DRM_FBDEV_EMULATION FRAMEBUFFER_CONSOLE BPF_SYSCALL CGROUP_BPF MEMCG PSI OVERLAY_FS FANOTIFY SECURITY_LANDLOCK INPUT_UINPUT; do
  grep -q "^CONFIG_$o=y" .config || { echo "MISSING CONFIG_$o after olddefconfig"; exit 1; }; done
grep -E '^CONFIG_(CC_VERSION_TEXT|RUSTC_VERSION_TEXT)=' .config
make -j"$(nproc)" > /sources/kernel_build_gnome51.log 2>&1 || { tail -40 /sources/kernel_build_gnome51.log; exit 1; }
make INSTALL_MOD_PATH=/sources/modules-gnome51 INSTALL_MOD_STRIP=1 modules_install >/dev/null
echo "BUILD OK: $(ls -l arch/x86/boot/bzImage)"
