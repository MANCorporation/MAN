#!/bin/bash
# MAN kernel: file systems on USB sticks, Windows/Mac disks and discs
# (exFAT, NTFS, HFS+, UDF, XFS, F2FS, all code pages). Incremental, run
# INSIDE the LFS chroot after 08-kernel-laptops.sh.
set -eo pipefail
export PATH=/usr/local/bin:/usr/bin:/usr/sbin:/tools/bin
cd /sources/linux-build-gnome51
cfg=./scripts/config
# exFAT and NTFS built in: USB sticks and the Windows partition always mount.
for o in EXFAT_FS NTFS3_FS NTFS3_LZX_XPRESS NTFS3_FS_POSIX_ACL; do $cfg --enable $o; done
for o in UDF_FS HFSPLUS_FS HFS_FS XFS_FS F2FS_FS CIFS; do $cfg --module $o; done
$cfg --set-str EXFAT_DEFAULT_IOCHARSET utf8
for o in $(sed -n 's/^config \(NLS_[A-Z0-9_]*\)$/\1/p' fs/nls/Kconfig); do
    case $o in NLS|NLS_DEFAULT) ;; NLS_UTF8|NLS_ASCII|NLS_CODEPAGE_437|NLS_ISO8859_1) $cfg --enable $o ;; *) $cfg --module $o ;; esac
done
make olddefconfig >/dev/null
for o in EXFAT_FS NTFS3_FS; do grep -q "^CONFIG_$o=y" .config || { echo "MISSING $o"; exit 1; }; done
for o in UDF_FS HFSPLUS_FS XFS_FS F2FS_FS NLS_CODEPAGE_852; do grep -q "^CONFIG_$o=[ym]" .config || { echo "MISSING $o"; exit 1; }; done
make -j"$(nproc)" > /sources/kernel_build_fs.log 2>&1 || { tail -40 /sources/kernel_build_fs.log; exit 1; }
rm -rf /sources/modules-fs
make INSTALL_MOD_PATH=/sources/modules-fs INSTALL_MOD_STRIP=1 modules_install >/dev/null
echo "BUILD OK: $(ls -l arch/x86/boot/bzImage); modules: $(find /sources/modules-fs -name '*.ko*' | wc -l)"
