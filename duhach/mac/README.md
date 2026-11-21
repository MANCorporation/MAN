# DUHACH! for Mac

Gets an Apple Silicon Mac (M1, M2, M3) ready to start MAN, with a calm step-by-step
guide in native macOS windows. Same three choices as `MAN-Mac/install-man.sh`:

1. Install the MAN boot loader (start MAN from the MAN USB stick or an external SSD)
2. Install the MAN boot loader and make space for MAN (asks the size)
3. Make space for MAN (boot loader already installed; asks the size)

## How it works

`DUHACH!.app` has no compiled code (so it can be built without a Mac). It has the same
pages and layout as DUHACH! for Windows (GTK 4/libadwaita): Welcome with "Checking This
Mac", Choose, Make Room (space bar + size), Ready (with the backup switch), Continue in
Terminal. The look is libadwaita's (header bar, boxed lists, pill buttons, light/dark),
drawn in HTML/CSS; real GTK on macOS would need building on a Mac.

- `Contents/MacOS/DUHACH`: runs `osascript -l JavaScript main.js <Resources>`; if that
  cannot start, the simpler guide in macOS dialogs (`guide.applescript`) runs instead.
- `main.js` (JavaScript for Automation): collects facts (model, chip, macOS version,
  APFS container size/free, charger, internet, MAN boot loader present), opens an
  NSWindow with a WKWebView showing `ui.html`, and handles its messages
  (`start`: writes a `.command` file and opens it in Terminal, no Automation permission
  needed; `quit`).
- `ui/index.html` -> `ui.html` (icon inlined by build-mac.sh). In a normal browser it runs
  as a demo; `#choose`, `#space`, `#ready`, `#dialog`, `#terminal` open a page directly.
- Terminal runs `MAN_CHOICE=<1|2|3> MAN_SIZE=<GB> sh MAN-Mac/install-man.sh`:
  MAN's copy of the Asahi installer (`dev-tools/apple/installer/patch-man-menu.py` takes
  the answers) with MAN's boot loader package. It shows the progress and asks only for
  the password; then the Mac shuts down for Apple's one-time step.

## Building

    duhach/mac/build-mac.sh            # release/DUHACH!-1.0.0-mac.zip (+ .sha256)
    duhach/mac/build-mac.sh DIR        # just DIR/DUHACH!.app (make-iso.sh: MAN_MAC partition)

The icon is made from `duhach/data/icons/.../com.mancorporation.Duhach.svg` by
`make-icns.py` (GdkPixbuf, PNG entries 128-1024 px).

## Gatekeeper

The app is not signed (no Apple Developer ID). From the MAN Live USB stick it opens
normally (files on a USB stick are not quarantined). Downloaded as a zip, macOS blocks
it the first time: System Settings > Privacy & Security > "Open Anyway".

## Not tested

main.js has not run on a real Mac yet (no macOS here). The pages were checked in
headless Firefox screenshots, the installer part with the answers on a simulated Mac
(`dev-tools/apple/installer/sim-mac.py`), the Terminal command with a path containing
spaces and "!" on Linux. The AppleScript guide (the fallback) worked on the M1 Pro.

## 1.1.0: installer package and menu bar (2026-11-21)

`build-mac.sh` makes `release/DUHACH-1.1.0-mac.pkg` (made on Linux by `make-pkg.py`:
a payload-free product archive whose postinstall copies DUHACH!.app to /Applications,
installs `/Library/LaunchAgents/com.mancorporation.Duhach.plist` and starts DUHACH! for
the logged-in user). Not signed: macOS asks once (Privacy & Security → Open Anyway).

DUHACH! then lives in the menu bar (`main.js --menubar`, started at each login):
- **Boot into MAN**: `bless --mount <MAN stub volume> --setBoot --nextonly --user … --stdinpass`
  (asks the login password), then a normal restart. If macOS refuses, Startup Disk opens.
- **Restore MAN Snapshot…**: reads `EFI/MAN/snapshots.json` on MAN's EFI partition (MAN
  writes it whenever its snapshots change), writes `EFI/MAN/restore-request` and boots
  MAN; `man-esp-requests.service` restores before the login screen and restarts.
  `EFI/MAN/restore-result` tells DUHACH! how it went.
- **More Settings…**: when the Mac starts (macOS / MAN), start-up options shared with
  MAN (`EFI/MAN/bootmenu.env`, read by GRUB: `man_verbose`, `man_debug`,
  `man_always_menu`), copy disk information, repair MAN's boot loader, start at login,
  and **Delete MAN** (24 hours after asking; Shift+Command held while clicking: at once).
  Deleting runs Asahi's uninstall steps and gives the space back to macOS
  (`diskutil apfs resizeContainer <macOS> 0`).
- **Install MAN…**: the installer window as before.

The disk logic is in `duhach-core.js`, tested on Linux with `node test-core.js`
(simulated diskutil plists). main.js (JXA) has NOT been run on a real Mac yet.
