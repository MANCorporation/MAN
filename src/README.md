# Patched GNOME packages for MAN

## gnome-control-center 1:51.0-1man1 (Settings)

Source: Debian `gnome-control-center 1:51.0-1` (same version as in the image),
unpacked in `gcc-51/`. MAN changes: `gcc-51/debian/patches/man-unsandbox-flatpak.patch`
(Apps panel only):

- **Sandbox** section on every Flatpak app's page: status row, **Unsandbox App…**
  (admin password → warning dialog → Proceed) and **Restore Sandbox**. It runs
  `pkexec /usr/libexec/man/man-flatpak-sandbox` (in man-system-tools, polkit
  action `com.man.flatpak.sandbox`, admin auth). Unsandboxing writes a
  system-wide Flatpak override (host + host-os + host-etc file systems, all
  devices, network, session and system bus, all sockets, devel/multiarch);
  restoring puts back the previous override file.
- Bug fix: with malcontent's app filtering disabled system-wide, upstream
  listed **no apps at all** in Settings → Apps. Now every app is listed.

Built in a VM against Debian unstable dev packages (`meson`, Debian's flags,
`-Dmalcontent=true`), binary `out/gnome-control-center`. Every adw/gtk/glib
symbol it uses exists in the image's libraries. The package in `debs/` is the
original Debian .deb with this binary, same dependencies, version `1man1`.

Note: Flatpak cannot switch its container off entirely; "unsandboxed" means
nothing is restricted, and the app can run host commands with
`flatpak-spawn --host`.

### 1:51.0-1man2 (2026-10-20)

Second patch `man-donate-man.patch`: Settings → About shows **Donate to MAN!**
("MAN is free, but it’s not free to maintain.") and the Donate button opens
https://donate.mancorporation.com/. Built with `gcc-build-in-vm.sh` (run as
root in a VM made from the image, after `apt-get build-dep gnome-control-center`
from sid), then the man1 .deb was repacked with the new binary as `debs/…man2`.

## PCD: epiphany-browser 51.1-1man1 (GNOME Web rebranded)

`epiphany/`: Debian source 51.1-1 plus `man-pcd-branding.patch`: name **PCD**
everywhere (window, menus, About, about: pages, crash page, privacy report,
sync, autofill labels), new PCD icon, About shows MAN with the Epiphany
copyright kept under Legal (GPL), desktop file/metainfo without GNOME,
distributor "MAN", no Help menu item and no help manual. The app id stays
`org.gnome.Epiphany` (internal only; keeps settings and profiles working).
`epiphany/build-in-vm.sh` builds the debs into `epiphany/debs/`. The old
`man-pcd.desktop` launcher is removed, so PCD is the only browser entry.

### 1:51.0-1man3 (2026-10-23): GNOME Tweaks options in Settings

`man-gnome-tweaks.patch`: new `panels/common/cc-man-tweaks.c` adds groups to
the panels (no separate Tweaks app): Appearance → Fonts (4 fonts, rendering,
hinting, antialiasing, scaling), Themes (cursor, icons, GTK 3 apps, sounds),
Window Titlebars (buttons, placement, double/middle/secondary click);
Multitasking → Windows (attach modal dialogs, center new windows, resize with
secondary click, window action key, raise on focus); Keyboard → More Keyboard
Options (overview key, extended input sources, Emacs input, Additional Layout
Options dialog with all XKB option groups, single/multiple choice from
evdev.xml); Mouse → General (middle-click paste, locate pointer); Power →
Laptop Lid (logind inhibitor started from ~/.config/autostart, only on
machines with a lid; `MAN_SETTINGS_FAKE_LID=1` shows it for testing); Apps →
Startup Apps dialog (~/.config/autostart). Incremental rebuild in the build
VM: `gcc-rebuild-in-vm.sh`. Tested in a VM: every row writes its key and the
desktop follows live (titlebar buttons, text scaling, xkb-options, lid
inhibitor appears/disappears, autostart file added/removed).

### 1:51.0-1man4 (2026-10-24): Updates panel, translated MAN strings

`man-updates-i18n.patch`:

- **Settings → Updates** (`panels/man-updates/`, sidebar after System):
  checks for a newer MAN as soon as it opens by running
  `man-update-check` (man-system-tools 3.8.0, looks at the GitHub releases
  of MANCorporation/MAN), shows the release notes, downloads the `.upd`
  with progress and Cancel, then opens MAN Update (which verifies the
  signature and installs). Switch "Check for Updates Automatically"
  (GSettings `com.man.Updates auto-check`, used by the daily user timer
  `man-update-check.timer`), "Last Checked", "Install an Update File…".
- MAN's own strings (Tweaks options, Sandbox, Donate, Updates) use the
  `man-settings` gettext domain (`panels/common/cc-man-i18n.h`, falls back
  to Settings' catalog); the Tweaks choice labels (N_) were never
  translated before and now are. Translations for 71 catalogs are in
  `man-settings-po/` (`tool.py import LANG tx/LANG.txt`, machine-written,
  native review welcome); `repack-gcc-deb.sh man4` builds the .deb with the
  binary, the `man-settings.mo` files and the translated panel desktop file.

## 1:51.0-1man5 (MAN 10.9, 2026-10-24)

`debian/patches/man-updates-in-settings.patch`: Settings → Updates installs
the update itself, no separate MAN Update window. After the download (or
"Install an Update File…") the file is checked by `man-installer-backend
update_inspect` (unprivileged), then License → Install Location (only when
there is more than one disk) → Summary → snapshot question → Preparing
(`pkexec … update_stage`) → Restart (30 s countdown, Restart Later) are
subpages. Every page keeps its main button in a fixed bottom bar; only the
content scrolls. Preparing keeps running when the panel is left and is shown
again on return (notification if the panel is gone). Strings shared with MAN
Update come from the `man-installer` catalog (`M_()`), one new string in
`man-settings` (`man-settings-po/new-man5.py`). Needs json-glib.
Pitfall: AdwStatusPage inside a preferences page lost its title after a state
change (own scroller), so the panel uses a plain box.

## 1:51.0-1man6 (MAN 10.10)

`debian/patches/man-updates-beta.patch`: "Beta Updates" switch in Settings →
Updates (GSettings `com.man.Updates beta-updates`, checks again when changed
after `g_settings_sync()` so `man-update-check` reads the new value) and a
"This is a TEST version" line when the offered version is a TEST build
(`test=true` from man-update-check). Three new strings
(`man-settings-po/new-man6.py`).
