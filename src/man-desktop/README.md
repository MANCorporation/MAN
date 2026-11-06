# man-desktop-shell: MAN's Dock, Blur, Window Tiling and system dialog look

`build.py SOURCES debs/man-desktop-shell_1.2_all.deb` turns two GPL GNOME Shell programs (Blur, Window Tiling) into
parts of MAN: names, settings schemas (`com.man.desktop.*`), D-Bus names and menus become MAN's,
their preference windows are removed, and Settings has the options (`cc-man-tweaks.c`, Appearance
and Multitasking). `check_no_names` fails the build if a name is left outside license files.
Their licenses stay in force: each folder keeps `COPYING`, `NOTICE` and the upstream copyright
comments, and `THIRD-PARTY-NOTICES.md` names them. Never remove those; source must stay available.

They still load through GNOME Shell's extension loader (system extensions in
`/usr/share/gnome-shell/extensions`), as that is the only supported way to change the Shell without
carrying patches to it. The image hides the Extensions app. `man-look` is MAN's own stylesheet.

`man-dock/` is MAN's own dock, written for MAN. On the desktop it shows a second copy of the Shell's
own dash (same icons, menus and look, see-through for the blur), hides when a window is in its way and
comes back at the screen edge (no pressure: VMs, tablets and touchscreens cannot do that). In the
Activities view it fades away and the Shell's dash, untouched, shows in the same place: the view
lays itself out as without a dock, also in always-show mode (`_updateWorkAreaBox`).

MAN's other changes beyond names: the dock's blur
is live (`static-blur=false`), and Window Tiling is off until turned on in Settings.

`upstream/` (not in git, made from these sources, needed for the source offer):
- `blur-my-shell` https://github.com/aunetx/blur-my-shell `309ebdca21`
- `tilingshell` https://github.com/domferr/tilingshell tag 18.0 (TypeScript source)
- `tilingshell-release`: its release zip of 18.0, which `build.py` uses

The default blur is stronger (radius 60 instead of 30).
