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
