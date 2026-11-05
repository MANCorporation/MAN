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

`upstream/` (not in git, made from these sources, needed for the source offer):
- `dash-to-dock` https://github.com/ubuntu/dash-to-dock `2dfae3474d`
- `blur-my-shell` https://github.com/aunetx/blur-my-shell `309ebdca21`
- `tilingshell` https://github.com/domferr/tilingshell tag 18.0 (TypeScript source)
- `tilingshell-release`: its release zip of 18.0, which `build.py` uses
