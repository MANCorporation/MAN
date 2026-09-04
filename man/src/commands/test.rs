//! `man test` — boot the built MAN image in QEMU.

use crate::cli::Arch;
use crate::utils;
use clap::Parser;
use colored::*;
use std::path::Path;

/// Arguments for the `test` subcommand.
#[derive(Parser, Debug)]
pub struct TestArgs {
    /// Target architecture to test.
    #[arg(short, long, value_enum, default_value = "x86_64")]
    pub arch: Arch,

    /// Enable KVM (hardware acceleration).
    #[arg(long)]
    pub kvm: Option<bool>,

    /// Amount of RAM in MB (default 4096 — COSMIC desktop needs at least 4 GiB).
    #[arg(short = 'm', long, default_value_t = 4096)]
    pub memory: u32,

    /// Number of virtual CPUs (default 4).
    #[arg(short = 'c', long, default_value_t = 4)]
    pub cpus: u8,

    /// Forward SSH port for headless access (e.g. `-p 2222:2222`).
    #[arg(short, long)]
    pub ssh_port: Option<u16>,

    /// Boot the UEFI El Torito ISO instead of the development disk.
    #[arg(long)]
    pub iso: bool,
}

pub fn run(args: &TestArgs, project_root: &Path) -> anyhow::Result<()> {
    let arch = args.arch;
    println!(
        "\n{}",
        format!("Booting MAN ({}) in QEMU…", arch.name())
            .bold()
            .cyan()
    );

    let output_dir = utils::project_path(project_root, format!("output/{}", arch.output_dir()));
    let images_dir = output_dir.join("images");

    if !images_dir.exists() {
        anyhow::bail!(
            "No build found at {:?}. Run `man build --arch {}` first.",
            images_dir,
            arch.name()
        );
    }

    // Build QEMU command based on architecture
    let qemu_bin = arch.qemu_binary();

    let mut cmd_args: Vec<String> = vec![];

    // Machine + CPU
    // usb=off removes the default UHCI controller that we replace with the
    // explicit qemu-xhci below, avoiding a redundant IRQ line and PCI slot.
    let machine = format!("{},usb=off", arch.qemu_machine());
    cmd_args.push("-machine".to_string());
    cmd_args.push(machine.to_string());

    #[cfg(target_os = "macos")]
    {
        // HVF can only accelerate a guest matching the Mac's host
        // architecture. Apple Silicon must emulate x86-64 with TCG (and an
        // Intel Mac must do the same for AArch64), otherwise QEMU reports an
        // invalid/unsupported CPU model before the firmware can start.
        let native_guest = match arch {
            Arch::Aarch64 => cfg!(target_arch = "aarch64"),
            Arch::X86_64 => cfg!(target_arch = "x86_64"),
        };
        cmd_args.push("-accel".to_string());
        cmd_args.push(if native_guest { "hvf" } else { "tcg" }.to_string());
    }
    #[cfg(not(target_os = "macos"))]
    if args.kvm.unwrap_or(false) {
        cmd_args.push("-accel".to_string());
        cmd_args.push("kvm".to_string());
    }

    // Memory
    cmd_args.push("-m".to_string());
    cmd_args.push(args.memory.to_string());

    // CPUs
    cmd_args.push("-smp".to_string());
    cmd_args.push(args.cpus.to_string());

    match arch {
        Arch::Aarch64 => {
            cmd_args.push("-cpu".to_string());
            #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
            // HVF can only execute the host CPU model. Named ARM cores such
            // as cortex-a72 are TCG models and QEMU rejects them with HVF.
            // pmu=off avoids virtualizing a PMU that COSMIC never polls, eliminating
            // unnecessary trap exits on every vCPU round-trip.
            cmd_args.push("host,pmu=off".to_string());
            #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
            cmd_args.push("cortex-a57,pmu=off".to_string());

            // EFI firmware (for MAN Boot Manager)
            cmd_args.push("-bios".to_string());
            cmd_args.push(arch.qemu_firmware().to_string());

            if args.iso {
                let iso_img = output_dir.join("man-aarch64.iso");
                if !iso_img.exists() {
                    anyhow::bail!("UEFI ISO not found. Run `man iso --arch aarch64` first.");
                }
                // ARM virt has no IDE CD-ROM. Its UEFI firmware discovers the
                // El Torito image when the CD is attached via VirtIO SCSI.
                cmd_args.push("-device".to_string());
                cmd_args.push("virtio-scsi-pci,id=scsi0".to_string());
                cmd_args.push("-drive".to_string());
                cmd_args.push(format!(
                    "file={},format=raw,if=none,media=cdrom,id=cd0",
                    iso_img.display()
                ));
                cmd_args.push("-device".to_string());
                cmd_args.push("scsi-cd,drive=cd0,bus=scsi0.0".to_string());

                // Give the live installer a safe, sparse QEMU-only target.
                // It is intentionally separate from the development disk.
                let install_img = images_dir.join("man-install-target.img");
                if !install_img.exists() {
                    let file = std::fs::File::create(&install_img)?;
                    file.set_len(8 * 1024 * 1024 * 1024)?;
                }
                cmd_args.push("-drive".to_string());
                cmd_args.push(format!(
                    "file={},format=raw,if=virtio,cache=writeback,discard=unmap",
                    install_img.display()
                ));
            } else {
                // ESP + ext2 rootfs, booted through MAN's UEFI boot manager.
                let disk_img = images_dir.join(format!("man-{}-disk.img", arch.name()));
                if !disk_img.exists() {
                    anyhow::bail!(
                        "Disk image not found in {:?}. Build with `man build --arch aarch64` first.",
                        images_dir
                    );
                }
                cmd_args.push("-drive".to_string());
                cmd_args.push(format!(
                    "file={},format=raw,if=virtio,cache=writeback,discard=unmap",
                    disk_img.display()
                ));
            }

            // Attach explicit USB input devices.  `usb-tablet` is an absolute
            // pointing device, which Wayland/libinput can use without relying
            // on QEMU's relative mouse-capture mode.
            cmd_args.push("-device".to_string());
            cmd_args.push("qemu-xhci,id=usb".to_string());
            cmd_args.push("-device".to_string());
            cmd_args.push("usb-kbd".to_string());
            cmd_args.push("-device".to_string());
            cmd_args.push("usb-tablet".to_string());

            // Virtio GPU (for framebuffer display on the GUI window)
            cmd_args.push("-device".to_string());
            cmd_args.push("virtio-gpu-pci,id=gpu".to_string());

            // Homebrew's ARM EDK2 does not contain VirtioGpuDxe. ramfb gives
            // firmware and MAN Boot Manager a GOP framebuffer; Linux/COSMIC then uses
            // the faster VirtIO GPU above. QEMU switches to the active scanout.
            cmd_args.push("-device".to_string());
            cmd_args.push("ramfb".to_string());

            // Network (e1000 + user-mode, with SSH port forwarding)
            cmd_args.push("-device".to_string());
            cmd_args.push("e1000,netdev=net0".to_string());
            cmd_args.push("-netdev".to_string());
            cmd_args.push("user,id=net0,net=10.0.2.0/24,hostfwd=tcp::2222-:22".to_string());

            // Virtio balloon — lets the guest return freed pages to the host,
            // keeping memory pressure low when the desktop is idle.
            cmd_args.push("-device".to_string());
            cmd_args.push("virtio-balloon-pci".to_string());

            // Serial console + QEMU monitor
            cmd_args.push("-serial".to_string());
            cmd_args.push("file:/tmp/qemu-serial.log".to_string());
            cmd_args.push("-monitor".to_string());
            cmd_args.push("tcp:127.0.0.1:4444,server,nowait".to_string());
        }
        Arch::X86_64 => {
            // Pass through the host CPU model so the guest sees the same
            // instruction-set extensions (SSE, AVX, etc.) as the host. This
            // prevents QEMU from emulating instructions one-by-one with TCG
            // or falling back to a generic x86-64 model under HVF/KVM.
            cmd_args.push("-cpu".to_string());
            cmd_args.push("host".to_string());

            // The release ISO lives beside the images directory. Honour
            // --iso explicitly instead of silently preferring a stale disk.
            let uefi_disk = images_dir.join("man-x86-64-disk.img");
            let iso_img = output_dir.join("man-x86-64.iso");
            let disk_img = images_dir.join("rootfs.ext2");

            // Homebrew QEMU 11's x86 EDK2 image is a full pflash volume and
            // is too large for the legacy -bios loader.
            cmd_args.push("-drive".to_string());
            cmd_args.push(format!(
                "if=pflash,format=raw,readonly=on,file={}",
                arch.qemu_firmware()
            ));

            if args.iso {
                if !iso_img.exists() {
                    anyhow::bail!("UEFI ISO not found. Run `man iso --arch x86-64` first.");
                }
                cmd_args.push("-cdrom".to_string());
                cmd_args.push(iso_img.to_string_lossy().to_string());
                cmd_args.push("-boot".to_string());
                cmd_args.push("d".to_string());

                let install_img = images_dir.join("man-install-target-x86-64.img");
                if !install_img.exists() {
                    let file = std::fs::File::create(&install_img)?;
                    file.set_len(8 * 1024 * 1024 * 1024)?;
                }
                cmd_args.push("-drive".to_string());
                cmd_args.push(format!(
                    "file={},format=raw,if=virtio,cache=writeback,discard=unmap",
                    install_img.display()
                ));
            } else if uefi_disk.exists() {
                cmd_args.push("-drive".to_string());
                cmd_args.push(format!(
                    "file={},format=raw,if=virtio,cache=writeback,discard=unmap",
                    uefi_disk.display()
                ));
            } else if disk_img.exists() {
                cmd_args.push("-drive".to_string());
                cmd_args.push(format!("file={},format=raw", disk_img.display()));
                cmd_args.push("-append".to_string());
                cmd_args.push("console=ttyS0 root=/dev/sda rw".to_string());
            } else {
                anyhow::bail!(
                    "No bootable image found in {:?}. Build with `man build` first.",
                    images_dir
                );
            }

            cmd_args.push("-device".to_string());
            cmd_args.push("qemu-xhci,id=usb".to_string());
            cmd_args.push("-device".to_string());
            cmd_args.push("usb-kbd".to_string());
            cmd_args.push("-device".to_string());
            cmd_args.push("usb-tablet".to_string());
            cmd_args.push("-device".to_string());
            cmd_args.push("virtio-vga,id=gpu".to_string());
            cmd_args.push("-device".to_string());
            cmd_args.push("e1000,netdev=net0".to_string());
            cmd_args.push("-netdev".to_string());
            cmd_args.push("user,id=net0,net=10.0.2.0/24,hostfwd=tcp::2223-:22".to_string());
            cmd_args.push("-serial".to_string());
            cmd_args.push("file:/tmp/qemu-x86_64-serial.log".to_string());

            // Virtio balloon — lets the guest return freed pages to the host.
            cmd_args.push("-device".to_string());
            cmd_args.push("virtio-balloon-pci".to_string());
        }
    }

    // Desktop images need a graphical window on macOS for keyboard, tablet and
    // GPU testing. CI/non-macOS environments retain a serial-only display.
    #[cfg(target_os = "macos")]
    {
        cmd_args.push("-display".to_string());
        cmd_args.push("cocoa".to_string());
    }
    #[cfg(not(target_os = "macos"))]
    cmd_args.push("-nographic".to_string());

    println!("  {} QEMU command: {} {}", "→".cyan(), qemu_bin, cmd_args.join(" "));
    println!("  {} Press Ctrl-A then X to exit QEMU", "💡".cyan());
    println!();

    // Launch QEMU (interactive, so we don't capture output)
    let mut command = std::process::Command::new(qemu_bin);
    for arg in &cmd_args {
        command.arg(arg);
    }

    // On macOS, set up the environment so QEMU can be found and libraries resolve
    #[cfg(target_os = "macos")]
    {
        let env = utils::setup_macos_env();
        for (key, value) in &env {
            command.env(key, value);
        }
    }

    let status = command.status()?;
    if !status.success() {
        eprintln!("{}", format!("QEMU exited with code {:?}", status.code()).red());
    }

    Ok(())
}
