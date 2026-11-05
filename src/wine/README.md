# MAN's patch to Wine: the desktop's file picker

Debian `wine 10.0~repack-12` with `man-native-dialogs.patch` (LGPL, like Wine):
GetOpenFileName, GetSaveFileName, IFileDialog (comdlg32) and SHBrowseForFolder
(shell32) show GNOME's file picker through zenity. Programs that customize the
dialog (hooks, templates, extra controls) keep Wine's dialog, as does
`MAN_WINE_DIALOGS=wine` or a system without zenity.

`build.sh` rebuilds the two DLLs for x86_64 and i386 on the PC (tools/mingw,
tools/hostbin) and packs `debs/man-wine-dialogs_*.deb`, which diverts Debian's
copies of comdlg32.dll and shell32.dll. Only those DLLs differ from Debian.

`test/pick.c` calls every dialog; `test/app.nsi` is an installer with shortcuts.
