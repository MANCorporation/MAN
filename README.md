# MAN Linux Distribution

A from-scratch Linux distribution built with **Rust** orchestration and **Toolchain**,
targeting both **x86_64** and **aarch64** (ARM64).

> "MAN" — because it's a *manual* Linux, built by hand and orchestrated by a tool
> you can understand, modify, and extend.

---

## Table of Contents

- [Quick Start](#quick-start)
- [Project Structure](#project-structure)
- [Components](#components)
- [Building](#building)
- [COSMIC Desktop](#cosmic-desktop)
- [Flatpak Applications](#flatpak-applications)
- [Testing](#testing)
- [UEFI Disk Images](#uefi-disk-images)
- [Distribution Targets](#distribution-targets)
- [Customization](#customization)
- [Requirements](#requirements)

---

## Quick Start

```bash
# 1. Set up the environment (install deps, clone Toolchain)
man setup

# 2. Build the complete distribution for x86_64 (including COSMIC)
man build --arch x86-64

# 3. Boot and test in QEMU
man test --arch x86-64
```

For ARM (aarch64), the same commands work:
```bash
man build --arch aarch64
man test --arch aarch64
```

You can also use the Makefile wrappers:
```bash
make setup
make build        # builds both architectures
make test         # boots x86_64 in QEMU
make disk         # creates UEFI disk images for both arches
```

### UEFI Boot (QEMU/UTM)

```bash
man build --arch aarch64
man disk --arch aarch64
# Then boot the .img in QEMU or UTM:
qemu-system-aarch64 -machine virt,accel=hvf -cpu host,pmu=off -m 4096 \
    -bios $HOME/.homebrew/share/qemu/edk2-aarch64-code.fd \
    -drive file=output/aarch64/images/man-aarch64-disk.img,format=raw,if=virtio,cache=writeback,discard=unmap \
    -device virtio-gpu-pci -serial mon:stdio
```

The QEMU runner combines `ramfb` for MAN Boot Manager graphics with VirtIO GPU
for the COSMIC session. Keep both devices when constructing a manual QEMU
command. Normal boot uses MAN's graphical splash; repeatedly pressing Alt
during early startup switches to verbose kernel and service output.

---

## Project Structure

```
MAN/
├── man/                    # Rust CLI orchestrator
│   ├── Cargo.toml          # Rust dependencies & metadata
│   └── src/
│       ├── main.rs         # CLI entry point
│       ├── cli.rs          # Arch enum & CLI types
│       ├── utils.rs        # Utility functions (find project root, run commands)
│       └── commands/
│           ├── setup.rs    # Setup subcommand (install deps, clone Toolchain)
│           ├── build.rs    # Build subcommand (run Toolchain make)
│           ├── test.rs     # Test subcommand (boot in QEMU)
│           ├── iso.rs      # ISO subcommand (create ISO image)
│           ├── disk.rs     # Disk subcommand (create UEFI disk image)
│           ├── clean.rs    # Clean subcommand
│           ├── config.rs   # Config subcommand
│           └── info.rs     # Info subcommand
├── configs/                # Toolchain defconfig files
│   ├── man-x86_64_defconfig
│   └── man-aarch64_defconfig
├── overlay/                # Root filesystem overlay (MAN branding & config)
│   ├── etc/
│   │   ├── hostname
│   │   ├── os-release
│   │   └── profile.d/man.sh
│   ├── var/motd
│   ├── root/
│   │   ├── .bashrc
│   │   └── .profile
│   └── usr/src/            # Rust source for distro utilities
├── scripts/                # Helper shell scripts
│   ├── create-efi-disk.sh  # Create UEFI-bootable disk image
│   ├── post-build.sh       # Toolchain post-build hook
│   ├── post-image.sh       # Toolchain post-image hook
│   ├── run-qemu.sh         # Standalone QEMU runner
│   └── bootloader/         # MAN Boot Manager binaries, drivers, and theme
├── toolchain/              # Toolchain source (cloned by `man setup`)
├── output/                 # Build output (generated)
│   ├── x86_64/
│   └── aarch64/
├── Makefile                # Top-level convenience targets
├── man.toml                # MAN configuration
└── README.md
```

---

## Components

| Component          | Description                                  | Language |
|--------------------|----------------------------------------------|----------|
| **CLI Orchestrator** | Manages builds, testing, and packaging    | **Rust** |
| **Toolchain**      | Cross-compilation toolchain & kernel builder | C/Make   |
| **Linux Kernel**   | The kernel (6.10.8)                        | C        |
| **glibc**          | GNU C Library                              | C        |
| **uutils/coreutils**| Rust reimplementation of core Unix utilities | **Rust** |
| **busybox**        | Rescue shell & additional utilities        | C        |
| **Rust Toolchain** | rustc + cargo on the target platform       | **Rust** |
| **bash + zsh**     | Login shells                               | C        |
| **openssh**        | SSH server/client                          | C        |
| **MAN Boot Manager** | Branded graphical UEFI boot experience      | C        |
| **COSMIC Epoch**   | Wayland compositor, shell, settings, and apps | **Rust** |
| **Flatpak**        | Sandboxed desktop application framework       | C        |
| **curl + wget**    | HTTP/HTTPS download and API tools            | C        |
| **git**            | Distributed version control                  | C        |
| **rsync**          | Fast file synchronization                    | C        |
| **htop**           | Interactive process viewer                   | C        |
| **bat, fd, ripgrep**| Rust CLI tools (cat, find, grep clones)     | **Rust** |
| **tree, lsof**     | File-tree viewer and open-file lister        | C        |
| **unzip, p7zip**   | Archive extraction (ZIP, 7z, XZ)             | C        |
| **jq, ca-cert**    | JSON parsing and TLS root store               | C        |
| **mpv**            | Wayland-compatible media player              | C        |
| **mupdf**          | Lightweight PDF and e-book viewer            | C        |
| **COSMIC Apps**    | Files, Terminal, Monitor, Screenshot, Settings | **Rust** |

---

## Building

### Setup

```bash
man setup                    # Default: x86_64, install deps, clone toolchain
man setup --install-deps     # Install Homebrew dependencies (QEMU, etc.)
man setup --all-archs        # Set up configs for both architectures
```

### Build

```bash
man build --arch x86-64      # Build x86_64 (default)
man build --arch aarch64     # Build aarch64
man build --arch x86-64 -j 8 # Use 8 parallel jobs
man build --arch x86-64 --clean # Clean first, then build
```

## COSMIC Desktop

MAN integrates System76's Rust-based COSMIC Epoch desktop directly from its
independent upstream crates. The compositor is built without systemd/logind;
the image uses BusyBox init, eudev, D-Bus, PipeWire, and libseat's builtin seat
backend. The session owns the compositor and starts the panel, applets,
launcher, workspaces, notifications, background, idle service, OSD, and
settings daemon.

```bash
# Build a complete rootfs, COSMIC desktop, Recovery image, and boot artifacts.
# The same command produces feature-equivalent x86_64 and aarch64 systems.
man build --arch aarch64

# Rebuild only COSMIC while developing components; this reuses the built rootfs.
man cosmic --arch aarch64 --no-image

# Rebuild a different pinned upstream release.
man cosmic --arch aarch64 --version epoch-1.6.0
```

The first COSMIC cross-build downloads all upstream submodules and compiles
each crate, so allow substantial disk space and time. Subsequent builds reuse
Cargo's per-component caches. Rust 1.93 or newer and the relevant Rust target
(`aarch64-unknown-linux-gnu` or `x86_64-unknown-linux-gnu`) are required.
Release binaries are stripped with Buildroot's target stripper before imaging.

For a graphical QEMU test, allocate at least 4 GiB RAM and keep the virtio GPU:

```bash
man test --arch aarch64 --memory 4096 --cpus 4
```

The ARM QEMU runner attaches both a USB keyboard and an absolute USB tablet,
so the pointer works directly in the COSMIC window. Click the QEMU window once
if macOS has not focused it. MAN's bundled `ICONS/` artwork is installed as the
`MAN` theme and mapped to COSMIC's launcher, application, and panel icons.
On Apple Silicon the runner uses QEMU's `-cpu host,pmu=off` with HVF; named ARM CPU
models are incompatible with that accelerator.

ARM boots through the graphical MAN Boot Manager. The runner supplies QEMU
`ramfb` as the UEFI GOP display and keeps VirtIO GPU for COSMIC. It presents
exactly two operating-system entries: **MAN** and **MAN Recovery**, with MAN as
the explicit default selection. Normal boot is quiet and
shows MAN's Rust framebuffer progress bar; repeatedly pressing either Alt key
during early userspace switches tty0 to verbose kernel/service output. The
live ISO opens full-screen MAN Utilities with Install MAN, Terminal, and
Diskulator. The live and recovery environments run `cosmic-comp` directly with
Utilities as the supervised shell, so the normal COSMIC panel, dock, launcher,
and workspace services are not started. Utilities ignores compositor close
requests—including Super+Q—and is restarted if it ever exits unexpectedly.
Installation is entirely graphical:
welcome, license agreement, destination selection, destructive confirmation,
installation progress, and completion/error pages. It never opens a terminal or
asks the user to type a command. Only the active installation page uses the clean,
centered single-progress-bar layout inspired by the supplied reference.

Diskulator is a separate native Rust/libcosmic application window. It is
available from MAN Utilities in Recovery and from the COSMIC application
library in normal MAN. Its device-sidebar,
action-toolbar, capacity-map layout is inspired by familiar disk tools while
remaining visually distinct and native to MAN. It displays physical disks,
partitions, filesystem types, sizes, allocation, and mount state. It can create MAN's
GPT layout (a recovery-sized FAT32 EFI System Partition plus an ext2 system partition),
create a single FAT32 or ext2 MBR volume, and run read-only First Aid checks for
FAT and ext filesystems. Destructive operations have a separate confirmation
page. Both the UI and privileged backends reject the running system disk and
mounted targets. Backend progress is reported to the app without exposing a
shell; detailed logs are stored in `/var/log/man-installer.log` and
`/var/log/man-diskulator.log`.

After a successful installation, the completion page offers an immediate
**Reboot Now** action and a visible 59-second countdown bar. The bar decreases
once per second and automatically reboots when it reaches zero. Remove the live
ISO or installation media before the countdown completes so firmware starts the
newly installed MAN system.

MAN Recovery is a separate, compressed operating-system image stored at
`/EFI/BOOT/man-recovery.cpio.gz` on the EFI partition. MAN Boot Manager loads it into RAM
with `man.recovery=1`; it does not mount the installed MAN root filesystem as
its own root. The image contains only the COSMIC compositor, Utilities,
Diskulator, Terminal, installer/disk backends, required runtime libraries, and
a compressed pristine MAN payload. This design lets Recovery reinstall or
repair the disk that stores it safely. The normal MAN entry boots the ext2
system partition and starts the complete COSMIC desktop.

After OOBE succeeds, normal boots are owned by `greetd` and System76's real
`cosmic-greeter`, authenticated through Linux-PAM. The direct libseat backend
keeps only the small COSMIC compositor side of the greeter session privileged;
the visible Greeter UI runs as the locked `cosmic-greeter` service account.
After authentication, `/usr/bin/man-user-session` starts COSMIC with the UID,
home directory, runtime directory, settings, and supplementary device groups
of the account created in OOBE. Recovery continues to boot MAN Utilities
directly and does not run the login manager.

On the first successful normal boot, MAN opens a full-screen, native Rust/
libcosmic first-run assistant. Its full path has 18 icon-led stages covering
the license, local administrator account, language, regional formats, keyboard,
light/dark mode, accent, wallpaper, desktop layout, dock behavior,
accessibility, privacy, updates, time zone, gestures, tiling, favorite apps,
and a final review. **Skip OOBE Personalization** is a safe quick path: it skips
the optional appearance and preference pages, but it does not bypass the
license or account creation. The assistant cannot be dismissed with the window
close shortcut and leaves no panel or dock exposed above its full-screen view.
The wallpaper page renders a 2×2 grid of real previews from MAN's bundled
COSMIC backgrounds. Selecting a preview updates the running desktop
immediately. Theme changes use a short full-window fade: the theme switches at
full opacity and the assistant fades smoothly back in, avoiding a harsh
light/dark flash.

Account creation is performed by `/usr/sbin/man-oobe-apply`; it creates a real
local user and administrator group memberships, stores only a SHA-512 password
hash in `/etc/shadow`, and never passes the password on a process command line.
The temporary password handoff is mode 0600 and removed immediately. Selected
COSMIC theme, wallpaper, and dock settings are applied to the active desktop.
Wallpaper selection is written to both COSMIC's watched background state and
its persistent configuration, then copied—with correct ownership—to the new
user's home. This makes the OOBE wallpaper survive the transition from first
boot to the authenticated desktop and later reboots. The complete set of
regional, accessibility, privacy, update, navigation, and
favorite-app choices is retained in `/etc/man-oobe.conf`. MAN writes
`/var/lib/man/oobe-complete` only after every mandatory operation succeeds, so
an interrupted or failed setup is offered again on the next boot. The OOBE and
its account backend are deliberately omitted from the independent Recovery OS.

For `man test --arch aarch64 --iso`, the runner creates an 8 GiB sparse
`images/man-install-target.img` so the complete installation flow can be tested
without touching the normal MAN development disk.

Runtime diagnostics are written to `/var/log/man-desktop.log` and
`/var/log/pipewire.log` in the guest. The generated ext2 image is 1 GiB and the
partitioned UEFI disk is sized from that image rather than using a fixed cap.

## Flatpak Applications

MAN includes Flatpak 1.16.6 with AppStream metadata, Bubblewrap/seccomp
sandboxing, OSTree repositories, XDG D-Bus proxying, FUSE, GPG verification,
and a system CA trust store. Flatpak applications are installed per user, so
they do not need administrator access and cannot modify MAN's base image.

The first authenticated COSMIC session adds Flathub in the background. If the
network is unavailable, MAN safely retries at the next login. Flatpak's user
and system export directories are included in `XDG_DATA_DIRS`, so installed
applications and their icons appear automatically in COSMIC's application
library.

**App Depot** is MAN's native Rust/libcosmic Flatpak store. It provides Flathub
browsing and search, AppStream descriptions and screenshots, application
details, per-user installation and removal, launching, repository management,
and individual or bulk updates. App Depot intentionally builds without
PackageKit, logind, or systemd integration: it manages sandboxed Flatpaks while
MAN's base system remains image-managed.

```bash
# Inspect the configured repository and find an application.
flatpak remotes
flatpak search <name>

# Install, update, run, or remove an application.
flatpak install flathub <application-id>
flatpak update
flatpak run <application-id>
flatpak uninstall <application-id>
```

Per-user data is stored below `~/.local/share/flatpak`; setup diagnostics are
written to `~/.local/state/man-flatpak.log`.

### Test

```bash
man test --arch x86-64       # Boot x86_64 in QEMU
man test --arch aarch64      # Boot aarch64 in QEMU (great on Apple Silicon)
man test --arch aarch64 --iso # Boot the graphical live installer ISO
man test --arch aarch64 --kvm # Enable hardware acceleration
man test --arch x86-64 -m 4096 -c 8  # 4GB RAM, 8 CPUs
```

### ISO Creation

```bash
man iso --arch x86-64      # Create bootable ISO (requires x86-64 build)
man iso --arch aarch64     # Create ARM64 UEFI El Torito ISO with initramfs
```

The legacy x86 ISO path uses `hdiutil` on macOS or `mkisofs`/`genisoimage` on
Linux. ARM64 uses `xorriso` because its EFI El Torito platform entry must be
marked as UEFI.

The ARM64 ISO contains an EFI El Torito boot image with MAN Boot Manager, the EFI-stub
kernel, and MAN's rootfs as an initramfs. `xorriso` and `mtools` are required.
The partitioned `output/aarch64/images/man-aarch64-disk.img` remains the faster
QEMU development image.

QEMU's ARM `virt` machine needs the ISO attached through VirtIO SCSI:

```bash
qemu-system-aarch64 -machine virt,accel=hvf -cpu host,pmu=off -m 4096 \
  -bios /Users/kristihack/.homebrew/share/qemu/edk2-aarch64-code.fd \
  -device virtio-scsi-pci,id=scsi0 \
  -drive file=output/aarch64/man-aarch64.iso,format=raw,if=none,media=cdrom,id=cd0 \
  -device scsi-cd,drive=cd0,bus=scsi0.0 \
  -device virtio-gpu-pci -device qemu-xhci,id=usb \
  -device usb-kbd -device usb-tablet
```

The pinned COSMIC Epoch superproject is cloned recursively to
`output/aarch64/build/cosmic-epoch` during `man cosmic`; it currently contains
all 28 upstream submodules. This is already inside the MAN project and is the
source tree used for cross-compilation.

The Rust test runner can attach the ISO correctly for you:

```bash
man test --arch aarch64 --iso --memory 4096 --cpus 4
```

---

## Testing

The `man test` command boots the built image in QEMU. On Apple Silicon (aarch64),
`--kvm` enables hardware acceleration for much faster emulation.

```bash
man test --arch aarch64 --kvm
```

You can also use the standalone script:
```bash
./scripts/run-qemu.sh x86_64 4096 4
```

---

## UEFI Disk Images

MAN boots through **MAN Boot Manager** and the kernel's built-in EFI stub
(`CONFIG_EFI_STUB=y`). The branded graphical menu provides normal and recovery
entries; the kernel EFI stub handles execution. No GRUB stage is required.

### Creating a Disk Image

```bash
man disk --arch aarch64    # Create UEFI disk image (aarch64)
man disk --arch x86-64     # Create UEFI disk image (x86-64)
```

Or via Make:
```bash
make disk-aarch64
make disk
```

This requires `mtools` (install via `brew install mtools` or
`man setup --install-deps`.). The disk image is also created automatically
during `man build` via the post-image hook (if `mtools` is available).

The resulting disk image has two MBR partitions:

| Partition | Size | Type | Contents |
|---|---|---|---|
| 1 (bootable) | Dynamic | FAT32 (ESP) | MAN Boot Manager, EFI-stub kernel, recovery image, drivers, and theme |
| 2 | ~923 MB | ext2 (Linux) | MAN rootfs |

### Booting

#### QEMU (aarch64 on Apple Silicon)

The bundled EDK2 firmware contains **only PEI-phase modules** — no DXE graphics
drivers (no `VirtioGpuDxe`, no `PciBusDxe`, no `GOP`). This means the firmware
The supplied runner adds `ramfb`, which gives UEFI a GOP framebuffer for the
graphical MAN Boot Manager, and VirtIO GPU for the Linux/COSMIC session.

```bash
qemu-system-aarch64 \
    -machine virt,accel=hvf -cpu host,pmu=off -m 4096 \
    -bios $HOME/.homebrew/share/qemu/edk2-aarch64-code.fd \
    -drive file=output/aarch64/images/man-aarch64-disk.img,format=raw,if=virtio,cache=writeback,discard=unmap \
    -device virtio-gpu-pci -serial mon:stdio
```

- `-device virtio-gpu-pci` → PCI GPU device. After the kernel boots, the Virtio
  GPU DRM driver (`CONFIG_DRM_VIRTIO_GPU=y`, built-in) detects it via PCI
  enumeration, initializes KMS, and creates a framebuffer. QEMU renders this
  framebuffer — the display shows the kernel console on a **black background**.
  **REQUIRED** for the display window — without it, no framebuffer is ever
  created and QEMU shows "Display output is not active."
  > **Why not `-device ramfb`?** `ramfb` is a QEMU-internal System-bus device —
  > the firmware can't detect it (no PCI enumeration without PciBusDxe), and
  > the kernel has no `ramfb` DRM driver. No framebuffer is ever set up.
  > `virtio-gpu-pci` is a PCI device the kernel's Virtio GPU DRM driver detects
  > and uses to create a framebuffer.
- `-serial mon:stdio` → firmware diagnostics and the kernel serial console.
  **Do not** use `-nographic` — it disables the display window entirely.

> The graphical menu requires `ramfb`; headless runs fall back to serial output.

For **headless** operation (no display window, serial terminal only):
```bash
qemu-system-aarch64 \
    -machine virt,accel=hvf -cpu host,pmu=off -m 4096 \
    -bios $HOME/.homebrew/share/qemu/edk2-aarch64-code.fd \
    -drive file=output/aarch64/images/man-aarch64-disk.img,format=raw,if=virtio,cache=writeback,discard=unmap \
    -device virtio-gpu-pci -serial mon:stdio -nographic
```

With `-nographic`, boot diagnostics and kernel output go to the serial terminal.
The system boots normally via the 5-second `timeout`.

#### QEMU (x86-64)

```bash
qemu-system-x86_64 \
    -machine q35,accel=hvf -m 4096 \
    -bios $HOME/.homebrew/share/qemu/edk2-x86_64-code.fd \
    -drive file=output/x86_64/images/man-x86_64-disk.img,format=raw,if=virtio,cache=writeback,discard=unmap \
    -nographic
```
> On x86-64 the `q35` machine provides a built-in VGA device — no extra
> `-device` flag is needed (the firmware's VGA driver handles the GOP
> framebuffer).

#### Headless (serial-only, no display window)

If you are running QEMU on a headless server (no display), add `-nographic`
in addition to `-device virtio-gpu-pci` (the GPU device is still needed so the
kernel's Virtio GPU DRM driver can set up a framebuffer after boot):

```bash
qemu-system-aarch64 \
    -machine virt,accel=hvf -cpu host,pmu=off -m 4096 \
    -bios $HOME/.homebrew/share/qemu/edk2-aarch64-code.fd \
    -drive file=output/aarch64/images/man-aarch64-disk.img,format=raw,if=virtio,cache=writeback,discard=unmap \
    -device virtio-gpu-pci -serial mon:stdio -nographic
```

With `-nographic`, boot diagnostics and kernel output go to the serial terminal.
The system boots normally via the 5-second `timeout`.

#### UTM (macOS GUI)

In UTM, select the **Virtio GPU** display and enable **3D Acceleration** in
 the VM hardware settings. MAN includes the Virtio GPU DRM kernel driver and
 Mesa virgl backend, so COSMIC can use the VM's accelerated renderer instead
 of falling back to software rendering:

1. Open UTM → Create VM → Virtualize → ARM64 (aarch64)
2. In **System**, select **UEFI** firmware (not "Emulate" / VIRT)
3. In **Drives**, add a **Removable** drive and select `man-aarch64-disk.img`
4. Boot — MAN Boot Manager shows the normal and recovery choices.
   After the kernel boots (~2-3 seconds), the display shows the kernel
   console on a **black background**, then the login prompt.

### Troubleshooting: "Display output is not active"

This message is **not** from the Linux kernel or UEFI firmware — it is
printed by **QEMU's display backend** (the Cocoa/SDL/GTK window, **not** the
serial console). You can confirm this by searching the QEMU binary:

```bash
strings $(which qemu-system-aarch64) | grep "Display output is not active"
```

The string lives in QEMU's own binary (alongside `"Guest has not initialized
the display (yet)."` and `"Guest display has been unplugged"`). It appears
when QEMU's display window is open but **no framebuffer device** has been
provided to the Guest — the display surface has nothing to render.

**Root cause (and fix):**

The bundled EDK2 firmware (`edk2-stable202408-prebuilt`) contains **only
PEI-phase modules** — it has **zero DXE drivers** (no `VirtioGpuDxe`, no
`PciBusDxe`, no `ConOutDxe`, no `GOP`). This means the firmware cannot set up
any graphics framebuffer by itself. The MAN runner therefore adds `ramfb` for
the boot menu; without it, QEMU's display backend can remain inactive until
Linux initializes VirtIO GPU.

`-device ramfb` does **not** fix this: `ramfb` is a System-bus device that the
firmware cannot enumerate (no PCI bus driver), and the kernel has no `ramfb`
DRM driver. With `ramfb` alone, no framebuffer is ever created.

The fix has two parts:
1. **`-device virtio-gpu-pci`** in the QEMU command — creates a PCI GPU device
   visible to both the firmware (for disk boot via PCI enumeration) and the
   kernel.
2. **Kernel with `CONFIG_DRM_VIRTIO_GPU=y`** (built-in) and **no `nomodeset`** —
   after the kernel boots, the Virtio GPU DRM driver detects the PCI device,
   initializes KMS, and creates a framebuffer. QEMU's display backend renders
   this framebuffer — the display shows the kernel console on a **black
   background**.

| Condition | Result |
|---|---|
| QEMU display window open, **no display device** | ❌ "Display output is not active." until Linux initializes a display |
| QEMU display window open, **`-device ramfb` only** | ❌ Same — no firmware driver, no kernel driver, no framebuffer |
| QEMU display window open, **`-device virtio-gpu-pci`** + kernel DRM driver | ✅ Display shows kernel console on **black background** after boot |
| `-nographic` (no display window) | ✅ No message (display backend disabled), serial on terminal |

**You MUST boot with `-device virtio-gpu-pci`** (and **without** `-nographic`
if you want to see the display):

```bash
qemu-system-aarch64 -machine virt,accel=hvf -cpu host,pmu=off -m 4096 \
    -bios $HOME/.homebrew/share/qemu/edk2-aarch64-code.fd \
    -drive file=output/aarch64/images/man-aarch64-disk.img,format=raw,if=virtio,cache=writeback,discard=unmap \
    -device virtio-gpu-pci -serial mon:stdio
```

Additionally, we **removed `video=efifb:off`** from the kernel command line.
On aarch64 the firmware provides no GOP framebuffer, so `efifb` finds nothing
and is harmless. The kernel's DRM virtio-gpu driver handles the display after
boot instead.

### How It Works

The boot chain is:

1. **UEFI firmware** (EDK2, PEI phase only — no DXE graphics drivers) scans the
   disk for `\EFI\BOOT\BOOTAA64.EFI` (MAN Boot Manager)
2. **MAN Boot Manager** displays the branded graphical menu (5-second timeout):
   - "MAN" — normal boot
   - "MAN Recovery" — independent recovery and installation environment
3. **Kernel EFI stub** (`CONFIG_EFI_STUB=y`) loads the kernel as an EFI application,
   using the command line from its EFI compatibility configuration. The
   kernel opts are:

   | Option | Purpose |
   |---|---|
   | `console=ttyAMA0,115200` | Serial console on PL011 |
   | `earlyprintk` | Early kernel messages on serial |
   | _(no nomodeset)_ | KMS is needed so the Virtio GPU DRM driver initializes |
   | `loglevel=1` | Suppress all non-emergency messages |
   | _(video=efifb:off removed)_ | EFI framebuffer driver is harmless (firmware has no GOP) |

5. **Kernel boots** → PCI bus enumerated → `virtio-gpu-pci` detected →
   `CONFIG_DRM_VIRTIO_GPU=y` initializes → KMS sets up framebuffer →
   QEMU display window shows kernel console on **black background**
6. **ext2 rootfs** is mounted from `/dev/vda2`
7. **`init`** starts → login prompt

MAN Boot Manager assets are bundled in `scripts/bootloader/`:
- `BOOTAA64.EFI` / `BOOTX64.EFI` — architecture-specific EFI applications
- `drivers_aa64/ext2_aa64.efi` — ext2 filesystem driver
- `theme/` — MAN background and large normal/recovery tiles
- `man-boot.conf` — source boot-menu configuration

---

## Distribution Targets

MAN supports two target architectures:

| Architecture | QEMU Machine | Use Case                    |
|---|---|---|
| **x86_64** | `q35` (Q35 chipset) | Standard PCs, servers, cloud VMs |
| **aarch64** | `virt` (Cortex-A57) | Raspberry Pi 4, ARM servers, Apple Silicon |

### Image Formats

- **x86_64**: ISO 9660 (live CD), EXT2 (disk image), squashfs
- **aarch64**: ISO 9660, EXT2 (disk image), squashfs

---

## Customization

### Adding Packages

Edit the defconfig files in `configs/` to add or remove packages:

```bash
# Edit the Toolchain config interactively
make O=output/x86_64 nconfig
# Then save the custom config
make O=output/x86_64 savedefconfig BR2_DEFCONFIG=configs/man-x86_64_defconfig
```

### Rootfs Overlay

Files in `overlay/` are copied into the root filesystem during build:
- `overlay/etc/` — system configuration
- `overlay/root/` — root user's home directory
- `overlay/usr/src/` — source code for Rust utilities

### MAN Configuration

The `man.toml` file controls build settings:

```toml
[build]
jobs = 8           # parallel jobs
libc = "glibc"     # or "musl"

[qemu]
default_memory = 4096
default_cpus = 4
```

---

## Requirements

### Build Environment (on macOS)

- **Rust** 1.75+ (for the CLI orchestrator)
- **Homebrew** (for QEMU and build tools) or manual QEMU install
- **QEMU** 7.0+ (for testing images, includes OVMF/AAVS UEFI firmware)
- **mtools** (for creating UEFI disk images — `brew install mtools`)
- **Git** (for cloning Toolchain)
- **GNU Make** (for Toolchain builds)

### Target Requirements

- **x86_64**: Standard PC hardware (BIOS/UEFI) or QEMU
- **aarch64**: ARM64 hardware (Raspberry Pi 4, etc.) or QEMU virt machine

### Build Time

- First build: ~15–30 minutes (downloads and compiles everything)
- Subsequent builds: ~2–5 minutes (incremental)
- On Apple Silicon, aarch64 builds with `--kvm` are fastest
