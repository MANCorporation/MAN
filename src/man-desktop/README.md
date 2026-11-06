# man-desktop-shell: MAN's Dock, Blur, Window Tiling and system dialog look

`build.py SOURCES debs/man-desktop-shell_1.0_all.deb` turns three GPL GNOME Shell programs into
parts of MAN: names, settings schemas (`com.man.desktop.*`), D-Bus names and menus become MAN's,
their preference windows are removed, and Settings has the options (`cc-man-tweaks.c`, Appearance
and Multitasking). `check_no_names` fails the build if a name is left outside license files.
Their licenses stay in force: each folder keeps `COPYING`, `NOTICE` and the upstream copyright
comments, and `THIRD-PARTY-NOTICES.md` names them. Never remove those; source must stay available.

They still load through GNOME Shell's extension loader (system extensions in
`/usr/share/gnome-shell/extensions`), as that is the only supported way to change the Shell without
carrying patches to it. The image hides the Extensions app. `man-look` is MAN's own stylesheet.

MAN's changes beyond names: the dock has the shape, size, place and icons of GNOME's dash in the
Activities view, measured against it pixel by pixel (`dev-tools/vm/dock-vs-stock.sh`, `dockmatch.py`:
64 px icons, no shrink, radius 28, padding 12, side margin 12, no focus square, running dot in the
same place, colour #38383b at 80 % so the blur shows), in both modes
(`$dock_fixed_inner_margin: 0`). Reaching the screen edge is enough to show it (no pressure, which
VMs, tablets and touchscreens cannot do; the check uses the current pointer position and works after
switching always-show off live), its blur
is live (`static-blur=false`), and Window Tiling is off until turned on in Settings.

`upstream/` (not in git, made from these sources, needed for the source offer):
- `dash-to-dock` https://github.com/ubuntu/dash-to-dock `2dfae3474d`
- `blur-my-shell` https://github.com/aunetx/blur-my-shell `309ebdca21`
- `tilingshell` https://github.com/domferr/tilingshell tag 18.0 (TypeScript source)
- `tilingshell-release`: its release zip of 18.0, which `build.py` uses
