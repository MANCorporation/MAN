#!/bin/bash
# Asahi kernel for MAN on Apple Silicon (M1/M2/M3), 16K pages, Rust GPU driver.
# Source: lfs/apple/linux (git clone --depth 1 --branch asahi-7.3.3-2 AsahiLinux/linux)
# Config: MAN's ARM64 kernel config + arch/arm64/configs/asahi.config + boot-critical
# drivers built in (disk, keyboard) so the initramfs does not need them.
# Output: lfs/apple/kernel/{vmlinuz,config,System.map,dtbs/,lib/modules}
set -e
p=$(cd -- "$(dirname -- "$(readlink -f "$0")")/../.." && pwd)
L=$p/tools/llvm/root/usr/lib/llvm-19
export PATH=$L/bin:$p/tools/hostbin/root/usr/bin:$HOME/.cargo/bin:$PATH
export LIBCLANG_PATH=$L/lib LD_LIBRARY_PATH=$L/lib:$p/tools/llvm/root/usr/lib/x86_64-linux-gnu
export BISON_PKGDATADIR=$p/tools/hostbin/root/usr/share/bison M4=$p/tools/hostbin/root/usr/bin/m4
K=$p/lfs/apple/linux O=$p/lfs/apple/build OUT=$p/lfs/apple/kernel
M="make -C $K O=$O ARCH=arm64 LLVM=1 LOCALVERSION= -j$(nproc)"
mkdir -p $O
if [ ! -f $O/.config ] || [ "$1" = config ]; then
  cp $p/lfs/arm64/kernel/config $O/.config
  $M olddefconfig
  $K/scripts/kconfig/merge_config.sh -m -O $O $O/.config $K/arch/arm64/configs/asahi.config
  s() { $K/scripts/config --file $O/.config "$@"; }
  s --set-str LOCALVERSION "-asahi-man"
  for c in NVME_APPLE APPLE_SART APPLE_RTKIT_HELPER PCIE_APPLE PINCTRL_APPLE_GPIO I2C_APPLE \
           SPI_APPLE SPI_HID_APPLE_OF SPI_HID_APPLE_CORE HID_APPLE MFD_MACSMC GPIO_MACSMC APPLE_DART PHY_APPLE_ATC \
           TYPEC_TPS6598X SPMI_APPLE NVMEM_APPLE_SPMI APPLE_WATCHDOG; do s -e $c; done
  # Drivers that need Mac firmware from the EFI partition stay modules: the initramfs
  # (asahi-scripts) loads that firmware first. M2+ laptop keyboards (DockChannel/MTP),
  # ASMedia USB on desktops (xhci-pci).
  s -m APPLE_DOCKCHANNEL -m HID_DOCKCHANNEL -m USB_XHCI_HCD -m USB_XHCI_PCI -m USB_XHCI_PLATFORM -m USB_DWC3
  s -e DRM_SIMPLEDRM -e FB_EFI -d FB_SIMPLE
  $M olddefconfig
fi
$M rustavailable
$M Image.gz modules dtbs
rm -rf $OUT; mkdir -p $OUT
cp $O/arch/arm64/boot/Image.gz $OUT/vmlinuz.gz; gunzip -c $OUT/vmlinuz.gz > $OUT/vmlinuz; rm $OUT/vmlinuz.gz
cp $O/.config $OUT/config; cp $O/System.map $OUT/
$M INSTALL_MOD_PATH=$OUT INSTALL_MOD_STRIP=1 modules_install
$M INSTALL_DTBS_PATH=$OUT/dtbs dtbs_install
echo "done: $(cat $O/include/config/kernel.release)"
