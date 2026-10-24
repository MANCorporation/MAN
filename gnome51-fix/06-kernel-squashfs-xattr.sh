#!/bin/bash
# Incremental rebuild of the MAN kernel with SQUASHFS_XATTR (file capabilities
# on the live ISO). Run INSIDE the LFS chroot after 05-kernel-gnome51.sh.
set -eo pipefail
export PATH=/usr/local/bin:/usr/bin:/usr/sbin:/tools/bin
cd /sources/linux-build-gnome51
./scripts/config --enable SQUASHFS_XATTR
make olddefconfig >/dev/null
grep -q '^CONFIG_SQUASHFS_XATTR=y' .config
make -j"$(nproc)" > /sources/kernel_build_xattr.log 2>&1 || { tail -40 /sources/kernel_build_xattr.log; exit 1; }
echo "BUILD OK: $(ls -l arch/x86/boot/bzImage)"
