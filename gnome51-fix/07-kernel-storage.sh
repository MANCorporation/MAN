#!/bin/bash
# MAN kernel: storage controllers found on real PCs (Intel VMD / RST "RAID On",
# eMMC, UFS, USB UAS, more SATA/NVMe-adjacent). Incremental, run INSIDE the
# LFS chroot after 06-kernel-squashfs-xattr.sh.
set -eo pipefail
export PATH=/usr/local/bin:/usr/bin:/usr/sbin:/tools/bin
cd /sources/linux-build-gnome51
E(){ for o; do ./scripts/config --enable "$o"; done; }
E PCI_MSI VMD BLK_DEV_NVME NVME_HWMON \
  MMC MMC_BLOCK MMC_SDHCI MMC_SDHCI_PCI MMC_SDHCI_ACPI MMC_SDHCI_PLTFM MMC_CQHCI MMC_HSQ \
  SCSI_UFSHCD SCSI_UFSHCD_PCI SCSI_UFSHCD_PLATFORM SCSI_UFS_BSG \
  USB_UAS USB_STORAGE SATA_AHCI SATA_AHCI_PLATFORM ATA_PIIX SATA_SIL24 \
  SCSI_VIRTIO VIRTIO_BLK MEGARAID_SAS SCSI_MPT3SAS HYPERV_STORAGE VMWARE_PVSCSI \
  NLS_CODEPAGE_437 NLS_ISO8859_1 NLS_UTF8 NLS_ASCII
make olddefconfig >/dev/null
for o in VMD MMC_BLOCK MMC_SDHCI_PCI SCSI_UFSHCD USB_UAS; do grep -q "^CONFIG_$o=y" .config || { echo "MISSING $o"; exit 1; }; done
make -j"$(nproc)" > /sources/kernel_build_storage.log 2>&1 || { tail -40 /sources/kernel_build_storage.log; exit 1; }
make INSTALL_MOD_PATH=/sources/modules-storage INSTALL_MOD_STRIP=1 modules_install >/dev/null
echo "BUILD OK: $(ls -l arch/x86/boot/bzImage)"
