# MAN Boot Theme Assets

The boot theme deliberately uses the project's established MAN icon style.
No generated icon artwork or upstream boot-manager logo is displayed.

## `man-background.png`

Deterministic 1280×800 RGB image whose pixels are all pure black (`#000000`).
It replaces the EFI application's built-in banner/logo and gives the boot menu
a completely unbranded background.

## `man-system.png`

Nearest-neighbor 256×256 enlargement of `../icons/os_man.png`, the existing MAN
system-drive icon.

## `man-recovery.png`

Nearest-neighbor 256×256 enlargement of `../icons/manrecovery.png`, the existing
MAN recovery-drive icon derived from the project `ICONS` artwork.
