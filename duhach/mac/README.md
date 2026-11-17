# DUHACH! for Mac

Gets an Apple Silicon Mac (M1, M2, M3) ready to start MAN, with a calm step-by-step
guide in native macOS windows. Same three choices as `MAN-Mac/install-man.sh`:

1. Install the MAN boot loader (start MAN from the MAN USB stick or an external SSD)
2. Install the MAN boot loader and make space for MAN (asks the size)
3. Make space for MAN (boot loader already installed; asks the size)

## How it works

`DUHACH!.app` has no compiled code (so it can be built without a Mac):

- `Contents/MacOS/DUHACH`: runs `osascript guide.applescript <Resources>`.
- `guide.applescript`: checks Apple Silicon and the chip (M1-M3), shows Welcome,
  Before you start (Time Machine, charger, internet, password), the choice, the size
  (from `df`, macOS keeps 38 GB free), and what happens next. Then it writes a
  `.command` file and opens it in Terminal (no Automation permission needed):
  `MAN_CHOICE=<1|2|3> MAN_SIZE=<GB> sh MAN-Mac/install-man.sh`.
- `MAN-Mac/`: from `dev-tools/apple/make-usb.sh`: MAN's copy of the Asahi installer
  (with `installer/patch-man-menu.py`, which takes `MAN_CHOICE`/`MAN_SIZE`), MAN's boot
  loader package. Terminal shows the progress and asks only for the password; then the
  Mac shuts down by itself for Apple's one-time step (hold the power button, choose
  MAN, password).

The guide is plain ASCII on purpose (AppleScript text files and non-ASCII characters).

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

The guide's AppleScript has not run on a real Mac yet (no macOS here): the installer
part with the answers was tested on a simulated Mac (`dev-tools/apple/installer/sim-mac.py`),
the Terminal command with a path containing spaces and "!" on Linux.
