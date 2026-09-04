//! `man disk` — create a UEFI-bootable disk image from the built MAN rootfs.
//!
//! Creates a partitioned disk image (`.img`) with:
//!   - Partition 1: FAT32 ESP (~100 MB) containing the kernel as an EFI stub
//!     application (`\EFI\BOOT\BOOTAA64.EFI` for aarch64, `\EFI\BOOT\BOOTX64.EFI`
//!     for x86-64), MAN Boot Manager assets, and the kernel.
//!   - Partition 2: ext2 root filesystem (the MAN rootfs).
//!
//! Requires `sfdisk` (from Toolchain host tools) and `mtools` (Homebrew:
//! `brew install mtools`, or system package) to be available on PATH.

use crate::cli::Arch;
use crate::utils;
use clap::Parser;
use colored::*;
use std::path::Path;
use std::time::Instant;

/// Arguments for the `disk` subcommand.
#[derive(Parser, Debug)]
pub struct DiskArgs {
    /// Target architecture.
    #[arg(short, long, value_enum, default_value = "x86_64")]
    pub arch: Arch,

    /// Disk image size in MB (default 1024).
    #[arg(short, long, default_value = "1024")]
    pub size: u32,

    /// ESP (EFI System Partition) size in MB (default 100).
    #[arg(long, default_value = "100")]
    pub esp_size: u32,
}

pub fn run(args: &DiskArgs, project_root: &Path) -> anyhow::Result<()> {
    let arch = args.arch;
    println!(
        "\n{}",
        format!("Creating UEFI disk image for {}…", arch.name())
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

    // Verify required tools — check system PATH plus common locations
    // (Homebrew, Toolchain host tools) where these tools may live.
    #[cfg(target_os = "macos")]
    let homebrew_bin = {
        let home = std::env::var("HOME").unwrap_or_default();
        let candidates = [
            format!("{}/.homebrew/bin", home),
            "/opt/homebrew/bin".to_string(),
            "/usr/local/bin".to_string(),
        ];
        candidates
            .into_iter()
            .find(|p| Path::new(p).join("brew").exists())
    };

    let sfdisk_found = utils::command_exists("sfdisk")
        || images_dir
            .parent()
            .unwrap()
            .join("host/sbin/sfdisk")
            .exists()
        || {
            #[cfg(target_os = "macos")]
            {
                if let Some(ref _hb) = homebrew_bin {
                    // sfdisk is typically in Toolchain host tools, not Homebrew
                    images_dir
                        .parent()
                        .unwrap()
                        .join("host/sbin/sfdisk")
                        .exists()
                } else {
                    false
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                false
            }
        };
    if !sfdisk_found {
        anyhow::bail!(
            "sfdisk not found. It is provided by Toolchain's host tools.\n\
             Ensure the Toolchain build has completed."
        );
    }

    let mtools_found = {
        let found = utils::command_exists("mcopy") && utils::command_exists("mformat");
        #[cfg(target_os = "macos")]
        {
            if !found {
                if let Some(ref hb) = homebrew_bin {
                    let p = Path::new(hb);
                    found || (p.join("mcopy").exists() && p.join("mformat").exists())
                } else {
                    found
                }
            } else {
                found
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            found
        }
    };
    if !mtools_found {
        anyhow::bail!(
            "mtools not found. Install it:\n  \
             macOS:  brew install mtools\n  \
             Linux:  apt install mtools\n  \
             Or run: man setup --install-deps"
        );
    }

    // Locate kernel image
    let kernel_candidates: &[&str] = match arch {
        Arch::Aarch64 => &["Image"],
        Arch::X86_64 => &["bzImage", "vmlinuz"],
    };
    let kernel_path = kernel_candidates
        .iter()
        .map(|name| images_dir.join(name))
        .find(|path| path.exists())
        .unwrap_or_else(|| images_dir.join(kernel_candidates[0]));
    if !kernel_path.exists() {
        anyhow::bail!(
            "Kernel image ({}) not found in {:?}. Build first with `man build --arch {}`.",
            kernel_candidates.join(" or "),
            images_dir,
            arch.name()
        );
    }

    // Locate rootfs
    let rootfs_path = images_dir.join("rootfs.ext2");
    if !rootfs_path.exists() {
        anyhow::bail!(
            "rootfs.ext2 not found in {:?}. Build first with `man build --arch {}`.",
            images_dir,
            arch.name()
        );
    }

    // Locate the create-efi-disk.sh script
    let script_path = utils::project_path(project_root, "scripts/create-efi-disk.sh");
    if !script_path.exists() {
        anyhow::bail!("create-efi-disk.sh not found at {:?}", script_path);
    }

    let script_str = script_path.to_string_lossy().to_string();
    let kernel_str = kernel_path.to_string_lossy().to_string();
    let rootfs_str = rootfs_path.to_string_lossy().to_string();
    let images_str = images_dir.to_string_lossy().to_string();
    // Disk scripts and image filenames use the UEFI-facing x86-64 spelling;
    // Buildroot output directories keep their conventional x86_64 spelling.
    let arch_name = match arch {
        Arch::Aarch64 => "aarch64",
        Arch::X86_64 => "x86-64",
    };

    println!("  {} Kernel:     {}", "✓".green(), kernel_path.display());
    println!("  {} Rootfs:     {}", "✓".green(), rootfs_path.display());
    println!("  {} Output:     {}/man-{}-disk.img", "→".cyan(), images_str, arch_name);

    let start = Instant::now();

    // On macOS, set up the environment (PATH includes Homebrew, Toolchain host tools)
    #[cfg(target_os = "macos")]
    let env = {
        let mut env = utils::setup_macos_env();
        // Prepend Toolchain host tools to PATH so sfdisk, e2fsprogs, etc. are found
        let host_dir = output_dir.join("host");
        let host_bin = host_dir.join("bin").to_string_lossy().to_string();
        let host_sbin = host_dir.join("sbin").to_string_lossy().to_string();
        let existing_path = env.get("PATH").cloned().unwrap_or_default();
        env.insert("PATH".to_string(), format!("{}:{}:{}", host_sbin, host_bin, existing_path));
        env.insert("HOST_DIR".to_string(), host_dir.to_string_lossy().to_string());
        env
    };

    #[cfg(not(target_os = "macos"))]
    let env = {
        let mut env = std::collections::HashMap::new();
        let host_dir = output_dir.join("host");
        let host_bin = host_dir.join("bin").to_string_lossy().to_string();
        let host_sbin = host_dir.join("sbin").to_string_lossy().to_string();
        let existing_path = std::env::var("PATH").unwrap_or_default();
        env.insert("PATH".to_string(), format!("{}:{}:{}", host_sbin, host_bin, existing_path));
        env.insert("HOST_DIR".to_string(), host_dir.to_string_lossy().to_string());
        env
    };

    utils::run_command_with_env(
        "bash",
        &[
            &script_str,
            &kernel_str,
            &rootfs_str,
            &images_str,
            &arch_name,
        ],
        Some(project_root),
        Some(env),
        Some(utils::UNSET_ENV_VARS.to_vec()),
    )?;

    let elapsed = start.elapsed();
    let disk_path = images_dir.join(format!("man-{}-disk.img", arch_name));

    if disk_path.exists() {
        let size = std::fs::metadata(&disk_path)?.len();
        println!(
            "\n  {} UEFI disk image created: {} ({:.1} MB) in {:.1}s",
            "✓".green(),
            disk_path.display(),
            size as f64 / (1024.0 * 1024.0),
            elapsed.as_secs_f64()
        );

        // Show boot instructions
        match arch {
            Arch::Aarch64 => {
                println!("\n  {} Boot in QEMU (UEFI):", "💡".cyan());
                println!(
                    "    {} qemu-system-aarch64 -machine virt,accel=hvf -cpu host -m 4096 \\",
                    "→".cyan()
                );
                println!("      -bios $HOME/.homebrew/share/qemu/edk2-aarch64-code.fd \\");
                println!("      -drive file={},format=raw,if=virtio \\", disk_path.display());
                println!("      -device virtio-gpu-pci -device ramfb -serial mon:stdio");
                println!();
                println!(
                    "      {} ramfb supplies UEFI GOP for the graphical MAN Boot Manager menu;",
                    "!".yellow().bold()
                );
                println!("        VirtIO GPU supplies COSMIC's Linux DRM display.");
                println!();
                println!(
                    "    {} Headless (serial only, no display window — for SSH/automation):",
                    "💡".cyan()
                );
                println!("      qemu-system-aarch64 ... -device virtio-gpu-pci -device ramfb \\");
                println!(
                    "        -drive file={},format=raw -serial mon:stdio -nographic",
                    disk_path.display()
                );
                println!("      (MAN Boot Manager auto-boots via its five-second timeout)");
            }
            Arch::X86_64 => {
                println!("\n  {} Boot in QEMU (UEFI):", "💡".cyan());
                #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
                let accel = "tcg";
                #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
                let accel = "hvf";
                println!(
                    "    {} qemu-system-x86_64 -machine q35,accel={} -m 4096 \\",
                    "→".cyan(),
                    accel
                );
                println!("      -drive if=pflash,format=raw,readonly=on,file=$HOME/.homebrew/share/qemu/edk2-x86_64-code.fd \\");
                println!(
                    "      -drive file={},format=raw -device qemu-xhci \\",
                    disk_path.display()
                );
                println!("      -device usb-kbd -device usb-tablet -device virtio-vga");
            }
        }

        println!("\n  {} For UTM: select this .img as the boot disk, choose UEFI firmware in VM settings", "💡".cyan());
    } else {
        anyhow::bail!("Disk image was not created — check for errors above");
    }

    println!();
    Ok(())
}
