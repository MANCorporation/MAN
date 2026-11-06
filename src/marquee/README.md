# Marquee on MAN

Marquee (https://github.com/MANCorporation/Marquee, MAN Corporation's own) is built from the
commit in `build.sh`. MAN adds:

- **`marquee` command**: `man-installer/crates/man-gui/src/marquee.rs`. Opening the app asks:
  open the media centre, or share on this network. Sharing is an Adwaita window (libraries,
  folders and files, on/off), with no web browser.
- **`share-on-man.patch`** for `marquee-share`, which runs as the user service
  `marquee-share.service`:
  - no web page and no `/admin` addresses: its admin page accepted requests from any
    website the user opened (`Access-Control-Allow-Origin: *`, no check of the origin), so a
    website could list folders and add a library that shares them. Libraries change only
    through `~/.config/marquee-share/libraries.json`, which it watches;
  - hidden files and folders are never served, links cannot lead out of a shared folder,
    and only the library's kind of files is served.

The protocol is unchanged: UDP 48765 announcements and HTTP on 8765. Any Marquee on the
network can use it, without a password (the sharing window says so).

Packed by `man-installer/packaging/build-debs.sh` as package `marquee`.
