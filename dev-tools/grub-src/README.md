# MAN's GRUB (the boot menu)

MAN starts without a menu. Holding **Alt** (Option on a Mac keyboard) at
power-on shows a start-up disk picker like a Mac's: a row of large icons for
MAN, Windows (its version: 7, 8, 10, 11 ...), macOS (on Apple Silicon with the
start menu on, e.g. "macOS Golden Gate"), bootable USB drives and MAN Recovery.

| Key | In the picker |
|---|---|
| ← → | choose |
| Enter | start it |
| Ctrl+Enter | start it, and from now on start it by default (stored in `EFI/MAN/bootmenu.env` on the EFI partition) |
| Shift+Enter on MAN, or Esc | "MAN Advanced Options": the full GRUB list (other kernels, recovery mode, snapshots, a staged update, firmware settings) |

Pressing Esc early at power-on also opens the list. On the live USB stick the
picker always shows (5 s), with "Try or Install MAN"; Shift+Enter on it lists
the other ways to start the stick.

## Pieces

- `man-bootmenu.patch` (GPL-3.0-or-later, as GRUB) on Debian's grub2:
  - gfxmenu: `layout = "horizontal"` for the boot menu (icons in a row, titles
    below, `selected_marker` under the chosen one; sizes follow the screen:
    `icon_percent`, `icon_min`, `icon_max`, `large_item_font`)
  - normal: with `menu_layout=horizontal` Left/Right move; `menu_mod` tells an
    entry which modifier was held with Enter (shift, ctrl, alt)
  - EFI console: modifiers are reported with no key waiting (a held Alt);
    Shift with Enter
  - keystatus: `--wait MS`
  - EFI disks: `man_connect` starts all drives below the PCI root bridges and
    lists the disks again (USB sticks under "fast boot"); known disks keep
    their names
- `build.sh efi|arm64|iso-efi|iso-pc`: rootless build (tools from
  `tools/hostbin`). Versions must match what they run with: installed PCs
  2.14-3 (the PC image's GRUB files; its package list wrongly says 2.12),
  ARM/Apple 2.14-4, the PC ISO 2.12-9+deb13u2 (this computer's grub-mkrescue).
- `make-deb.sh amd64|arm64`: the `man-grub` package: modules in
  `/usr/lib/man/grub/<platform>`, `/usr/sbin/grub-install` (Debian's is
  diverted to `grub-install.debian`; MAN's modules are used unless
  `--directory` is given), `man-grub-refresh` (puts the new GRUB on the EFI
  partition after an upgrade or MAN update, no NVRAM writes), the theme
  `/usr/share/grub/themes/man`.
- `man-installer/data/grub/`: `06_man_bootmenu` (top of grub.cfg: Alt, Ctrl,
  views, USB scan), `49_man_bootmenu` (writes `/boot/grub/man-picker.cfg`),
  `bootmenu-lib.sh` (Windows version from bootmgfw.efi, macOS version noted by
  MAN's Mac installer, Recovery) - in man-system-tools.
- `dev-tools/bootmenu/`: `make-icons.py` + `make-theme.sh` (icons, selection
  box, fonts), `test-vm.sh` (the menu with sample entries in QEMU).

GRUB scripts have no `&&`, and functions cannot be defined inside `if` or
while they run: that is why the picker is its own file and entries call
`configfile` themselves.

## Rebuild

    dev-tools/grub-src/build.sh efi && dev-tools/grub-src/build.sh arm64
    dev-tools/grub-src/build.sh iso-efi && dev-tools/grub-src/build.sh iso-pc
    bash dev-tools/bootmenu/make-theme.sh
    dev-tools/grub-src/make-deb.sh amd64 && dev-tools/grub-src/make-deb.sh arm64

Testing in QEMU: holding a key is simulated with `dev-tools/vm/hold.py MON alt 15`
(re-pressed every 0.1 s like a keyboard's auto-repeat; start it before the
firmware, a press made earlier is lost).
