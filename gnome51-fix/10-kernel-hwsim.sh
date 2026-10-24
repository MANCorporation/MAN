#!/bin/bash
# MAN kernel: mac80211_hwsim (simulated Wi-Fi radios, for testing Wi-Fi in a VM).
# Incremental, run INSIDE the LFS chroot after 09-kernel-filesystems.sh.
set -eo pipefail
export PATH=/usr/local/bin:/usr/bin:/usr/sbin:/tools/bin
cd /sources/linux-build-gnome51
./scripts/config --module MAC80211_HWSIM
make olddefconfig >/dev/null
grep -q '^CONFIG_MAC80211_HWSIM=m' .config
make -j"$(nproc)" > /sources/kernel_build_hwsim.log 2>&1 || { tail -40 /sources/kernel_build_hwsim.log; exit 1; }
rm -rf /sources/modules-hwsim
make INSTALL_MOD_PATH=/sources/modules-hwsim INSTALL_MOD_STRIP=1 modules_install >/dev/null
echo "BUILD OK: $(ls -l arch/x86/boot/bzImage); modules: $(find /sources/modules-hwsim -name '*.ko*' | wc -l)"
