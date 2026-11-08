# Third-party software in MAN

MAN Corporation's own work in this repository is proprietary (see
[LICENSE](LICENSE)). Everything listed here belongs to others and is used
under its own license. Those licenses are not changed or limited by MAN's
license.

## 1. Rust libraries in MAN's programs

Compiled into MAN Installer, MAN Setup, Snapshots, MAN Update and
man-mkupd: [man-installer/THIRD-PARTY-RUST.txt](man-installer/THIRD-PARTY-RUST.txt).
Compiled into DUHACH!: [duhach/THIRD-PARTY-RUST.txt](duhach/THIRD-PARTY-RUST.txt).
All are permissive (MIT, Apache-2.0, BSD, Unicode, Unlicense). The files
list each library, its license and the full license texts, and are shipped
in the packages (`/usr/share/doc/man-*/copyright`) and in the DUHACH! zip.
Regenerate them after changing dependencies:

```bash
python3 dev-tools/rust-notices.py man-installer x86_64-unknown-linux-gnu
python3 dev-tools/rust-notices.py duhach x86_64-pc-windows-gnu
```

## 2. The MAN operating system (disk image, ISO, .upd updates)

The MAN system is a collection ("aggregate") of independent programs. Apart
from MAN's own packages (`man-installer`, `man-system-tools`), it consists of:

| Component | License | Source |
|---|---|---|
| Debian 13 / unstable packages (GNU tools, GNOME 51, systemd, Mesa, …) | each package's own license, see `/usr/share/doc/<package>/copyright` | https://sources.debian.org, https://snapshot.debian.org (exact versions: `dpkg -l` in MAN) |
| Linux kernel 7.3-rc7 | GPL-2.0-only | upstream https://kernel.org + MAN's configuration (`gnome51-fix/0*-kernel-*.sh`) |
| GNOME Settings `1:51.0-1manN` (patched) | GPL-2.0-or-later | `src/` (Debian source + `man-*.patch`) |
| GNOME Web 51.1-1manN, shipped as "PCD" (patched) | GPL-3.0-or-later | `src/epiphany/` (Debian source + `man-pcd-branding.patch`) |
| MAN Blur (`man-desktop-shell`): modified [Blur my Shell](https://github.com/aunetx/blur-my-shell) `309ebdca21` | GPL-3.0 | `src/man-desktop/` (`build.py`, `upstream/`) |
| MAN Window Tiling (`man-desktop-shell`): modified [Tiling Shell](https://github.com/domferr/tilingshell) 18.0 | GPL-3.0 | `src/man-desktop/` (`build.py`, `upstream/`) |
| GRUB 2 (boot menu, ISO, DUHACH! partition) | GPL-3.0-or-later | Debian `grub2` |
| BusyBox (ISO start-up, `iso/init` uses it) | GPL-2.0-only | Debian `busybox` 1:1.37.0-6 |
| Firmware (Wi-Fi, Bluetooth, GPU, sound, CPU microcode) | vendors' redistribution licenses, unmodified | Debian `firmware-*`, `/usr/share/doc/firmware-*/copyright` |
| Noto fonts | SIL Open Font License 1.1 | Debian `fonts-noto-*` |
| Flatpak apps installed by users | their own licenses | Flathub |

MAN Setup shows the main licenses before first use, and every installed
component's license is in `/usr/share/doc`.

### Source code offer (GPL / LGPL)

For every component under the GNU GPL or LGPL that MAN distributes in binary
form (in the ISO, the disk image or a `.upd` update), MAN Corporation will
give anyone who asks a complete copy of the corresponding source code, for
at least three years after the last distribution of that MAN version, for
no more than the cost of providing it. Ask at
https://github.com/MANCorporation/MAN/issues. MAN's changes to GPL programs
are in `src/` of this repository.

## 3. Libraries bundled with DUHACH! (Windows)

The DUHACH! zip contains unmodified DLLs and data from MSYS2 packages
(GTK 4, libadwaita, GLib, Pango, cairo, HarfBuzz, gdk-pixbuf, librsvg,
FreeType, libpng, zlib, the GCC runtime and others; versions in
`tools/gtk-win/pkgs`). GTK, GLib, libadwaita and other LGPL libraries are
linked dynamically, so they can be replaced. `package-windows.sh` puts
their license files (`licenses/`), a list of the exact package versions and
the source location (`THIRD-PARTY-NOTICES.txt`) into the zip.

## 4. Trademarks

Linux is a registered trademark of Linus Torvalds. GNOME is a trademark of
the GNOME Foundation, Debian of Software in the Public Interest, Inc.,
Windows of Microsoft Corporation. These names are used only to describe
compatibility and origin; MAN is not endorsed by these organisations. The
Debian logo is not used in MAN (the login screen shows the MAN logo).

## Resources

Resources (system monitor, https://github.com/nokyan/resources, GNU GPL 3) is Debian's package,
unchanged. Source: https://sources.debian.org/src/resources/.

## Marquee

Marquee is MAN Corporation's own (All Rights Reserved). It contains third-party material with its own
licences, listed in full in `/usr/share/doc/marquee/copyright`: among them carousel and layout logic
derived from JacksonDam/FirstRow (CC BY 4.0), Adwaita icons (CC BY-SA 3.0 / LGPL-3.0) and the Lato font
(SIL OFL 1.1). FFmpeg and yt-dlp are Debian's packages, separate programs under their own licences.

## Wine and ClamAV

Wine (GNU LGPL 2.1+) and ClamAV (GNU GPL 2) are Debian's packages. MAN changes two Wine libraries
(comdlg32, shell32): the patch and how to rebuild them are in `src/wine/`, and installed in
`/usr/share/doc/man-wine-dialogs/`. Sources: https://sources.debian.org/src/wine/ and
https://sources.debian.org/src/clamav/.

## Wallpapers

MAN ships only its own wallpaper set (`/usr/share/backgrounds/man/`, package man-live-config); GNOME's
set is not installed. Credits, also in `/usr/share/doc/man-live-config/copyright`:

- Golden Gate Bridge and Yosemite National Park: photos by Carol M. Highsmith, Carol M. Highsmith
  Archive, Library of Congress, Prints and Photographs Division. Public domain.
- Western Alps (towards the Western Alps from Grenoble): photo by Dktue, Wikimedia Commons, CC0 1.0.
- Hintersee (Berchtesgaden, Germany): source and licence to be confirmed before publishing.
