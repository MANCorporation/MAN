#!/bin/bash
# MAN kernel: drivers for real laptops and desktops (2026-10-22). Touchpads
# (I2C HID + Intel/AMD pin controllers), Wi-Fi, Bluetooth, audio codecs and
# Intel/AMD SOF, AMD / Intel Xe / NVIDIA graphics, webcams, card readers,
# laptop hotkeys, sensors, USB4. Drivers that need firmware are modules, so
# they load after the root file system (with /usr/lib/firmware) is mounted.
# Incremental: run INSIDE the LFS chroot after 07-kernel-storage.sh.
set -eo pipefail
export PATH=/usr/local/bin:/usr/bin:/usr/sbin:/tools/bin
cd /sources/linux-build-gnome51
cfg=./scripts/config

# Every option of the given Kconfig files: tristate -> m (or y with -y),
# bool -> y. Debug, test and tracing options are left alone.
all_of() {
    local want=m
    [ "$1" = -y ] && { want=y; shift; }
    awk '/^(menu)?config [A-Z0-9_]+/ { name = $2; next }
         name && /^\s+(tristate|bool)/ { print name, $1; name = "" }' "$@" |
    grep -v -E 'DEBUG|TEST|KUNIT|TRACE|TRACING|_DBG|WERROR|SPECTRAL|COMPILE|DEPRECATED|_LEDS$' |
    while read -r name type; do
        if [ "$type" = bool ] || [ $want = y ]; then $cfg --enable "$name"; else $cfg --module "$name"; fi
    done
}
Y(){ for o; do $cfg --enable "$o"; done; }
M(){ for o; do $cfg --module "$o"; done; }

# Touchpads and touchscreens: I2C controller, pin controllers, HID over I2C.
Y X86_INTEL_LPSS MFD_INTEL_LPSS MFD_INTEL_LPSS_PCI MFD_INTEL_LPSS_ACPI INTEL_IDMA64 \
  I2C_DESIGNWARE_CORE I2C_DESIGNWARE_PLATFORM I2C_DESIGNWARE_PCI I2C_DESIGNWARE_BAYTRAIL \
  PINCTRL PINCTRL_AMD GPIO_ACPI \
  HID HID_GENERIC I2C_HID I2C_HID_ACPI HID_MULTITOUCH HID_ASUS HID_LENOVO HID_LOGITECH HID_APPLE \
  MOUSE_PS2 MOUSE_PS2_ELANTECH MOUSE_PS2_ELANTECH_SMBUS MOUSE_PS2_SYNAPTICS MOUSE_PS2_SYNAPTICS_SMBUS MOUSE_PS2_SMBUS \
  MOUSE_ELAN_I2C MOUSE_ELAN_I2C_I2C MOUSE_ELAN_I2C_SMBUS MOUSE_SYNAPTICS_I2C \
  RMI4_CORE RMI4_I2C RMI4_SMB RMI4_F03 RMI4_F11 RMI4_F12 RMI4_F30 INPUT_TOUCHSCREEN
all_of -y drivers/pinctrl/intel/Kconfig
M HID_SENSOR_HUB HID_SENSOR_ACCEL_3D HID_SENSOR_ALS INTEL_ISH_HID AMD_SFH_HID INPUT_SOC_BUTTON_ARRAY HID_WACOM

# Laptop hotkeys, fans, battery extras.
Y X86_PLATFORM_DEVICES ACPI_WMI
M WMI_BMOF ASUS_WMI ASUS_NB_WMI EEEPC_WMI IDEAPAD_LAPTOP THINKPAD_ACPI HP_WMI DELL_WMI DELL_LAPTOP DELL_SMBIOS \
  ACER_WMI MSI_WMI MSI_LAPTOP TOSHIBA_ACPI SAMSUNG_LAPTOP INTEL_HID_EVENT INTEL_VBTN AMD_PMC AMD_HSMP \
  SENSORS_CORETEMP SENSORS_K10TEMP INTEL_PCH_THERMAL INT340X_THERMAL INTEL_POWERCLAMP
Y X86_INTEL_PSTATE X86_AMD_PSTATE

# Wi-Fi.
Y WLAN CFG80211 MAC80211 RFKILL
all_of drivers/net/wireless/intel/Kconfig drivers/net/wireless/intel/iwlwifi/Kconfig \
  drivers/net/wireless/realtek/Kconfig drivers/net/wireless/realtek/rtw88/Kconfig drivers/net/wireless/realtek/rtw89/Kconfig \
  drivers/net/wireless/realtek/rtl8xxxu/Kconfig \
  drivers/net/wireless/ath/Kconfig drivers/net/wireless/ath/ath9k/Kconfig drivers/net/wireless/ath/ath10k/Kconfig \
  drivers/net/wireless/ath/ath11k/Kconfig drivers/net/wireless/ath/ath12k/Kconfig \
  drivers/net/wireless/mediatek/Kconfig $(find drivers/net/wireless/mediatek/mt76 -name Kconfig) \
  drivers/net/wireless/broadcom/Kconfig drivers/net/wireless/broadcom/brcm80211/Kconfig
# Wired network cards common in laptops and desktops.
M IGC IGB R8169 ALX ATL1C TG3 USB_RTL8152 USB_NET_CDCETHER USB_NET_AX88179_178A

# Bluetooth.
Y BT_BREDR BT_LE BT_HS
M BT BT_RFCOMM BT_BNEP BT_HIDP
$cfg --enable BT_RFCOMM_TTY --enable BT_BNEP_MC_FILTER --enable BT_BNEP_PROTO_FILTER
all_of drivers/bluetooth/Kconfig

# Audio: HD-Audio codecs (Realtek, Conexant, Cirrus, HDMI ...) and SOF DSPs.
Y SND_HDA_INTEL SND_HDA_GENERIC
all_of $(find sound/hda -name Kconfig)
all_of sound/soc/sof/Kconfig sound/soc/sof/intel/Kconfig sound/soc/sof/amd/Kconfig \
  sound/soc/intel/Kconfig sound/soc/intel/boards/Kconfig sound/soc/amd/Kconfig
Y SND_SOC SND_SOC_SOF_TOPLEVEL SND_SOC_SOF_PCI SND_SOC_SOF_INTEL_TOPLEVEL SND_SOC_SOF_AMD_TOPLEVEL \
  SND_SOC_INTEL_SST_TOPLEVEL SND_SOC_INTEL_MACH SOUNDWIRE SND_INTEL_DSP_CONFIG
M SND_USB_AUDIO

# Graphics beyond i915/virtio: AMD, Intel Xe, NVIDIA (nouveau), old Radeon.
M DRM_AMDGPU DRM_RADEON DRM_XE DRM_NOUVEAU
Y DRM_AMD_DC DRM_AMDGPU_SI DRM_AMDGPU_CIK DRM_AMDGPU_USERPTR HSA_AMD

# Webcams, card readers, USB4/Thunderbolt.
Y MEDIA_SUPPORT_FILTER MEDIA_CAMERA_SUPPORT MEDIA_USB_SUPPORT
M MEDIA_SUPPORT VIDEO_DEV USB_VIDEO_CLASS MISC_RTSX_PCI MISC_RTSX_USB MMC_REALTEK_PCI MMC_REALTEK_USB USB4

make olddefconfig >/dev/null
for o in I2C_HID_ACPI HID_MULTITOUCH PINCTRL_TIGERLAKE MFD_INTEL_LPSS_PCI I2C_DESIGNWARE_PLATFORM; do
    grep -q "^CONFIG_$o=y" .config || { echo "MISSING (y) $o"; exit 1; }
done
for o in IWLWIFI IWLMVM RTW88_8822CE RTW89_8852BE ATH11K_PCI MT7921E BT_HCIBTUSB SND_HDA_CODEC_ALC269 \
         SND_SOC_SOF_INTEL_TGL DRM_AMDGPU USB_VIDEO_CLASS ASUS_NB_WMI; do
    grep -q "^CONFIG_$o=[ym]" .config || { echo "MISSING $o"; exit 1; }
done
make -j"$(nproc)" > /sources/kernel_build_laptops.log 2>&1 || { tail -40 /sources/kernel_build_laptops.log; exit 1; }
rm -rf /sources/modules-laptops
make INSTALL_MOD_PATH=/sources/modules-laptops INSTALL_MOD_STRIP=1 modules_install >/dev/null
echo "BUILD OK: $(ls -l arch/x86/boot/bzImage); modules: $(find /sources/modules-laptops -name '*.ko*' | wc -l)"
