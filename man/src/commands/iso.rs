//! `man iso` — create an ISO image from the built MAN rootfs.

use crate::cli::Arch;
use crate::utils;
use clap::Parser;
use colored::*;
use std::path::Path;

/// Arguments for the `iso` subcommand.
#[derive(Parser, Debug)]
pub struct IsoArgs {
    /// Target architecture.
    #[arg(short, long, value_enum, default_value = "x86_64")]
    pub arch: Arch,
}

pub fn run(args: &IsoArgs, project_root: &Path) -> anyhow::Result<()> {
    let arch = args.arch;
    if matches!(arch, Arch::Aarch64 | Arch::X86_64) {
        let output_dir = utils::project_path(project_root, format!("output/{}", arch.output_dir()));
        let target_dir = output_dir.join("target");
        let images_dir = output_dir.join("images");
        let (kernel_name, iso_name, script_arch) = match arch {
            Arch::Aarch64 => ("Image", "man-aarch64.iso", "aarch64"),
            Arch::X86_64 => ("bzImage", "man-x86-64.iso", "x86-64"),
        };
        let iso_path = output_dir.join(iso_name);
        let script = utils::project_path(project_root, "scripts/create-uefi-iso.sh");
        if !target_dir.exists() || !images_dir.join(kernel_name).exists() {
            anyhow::bail!("Build MAN/{} before creating its UEFI ISO", arch.name());
        }
        utils::run_command(
            "bash",
            &[
                script.to_str().unwrap(),
                target_dir.to_str().unwrap(),
                images_dir.to_str().unwrap(),
                iso_path.to_str().unwrap(),
                script_arch,
            ],
            Some(project_root),
        )?;
        println!("\n  {} UEFI ISO created: {}", "✓".green(), iso_path.display());
        return Ok(());
    }
    println!(
        "\n{}",
        format!("Creating MAN ISO for {}…", arch.name())
            .bold()
            .cyan()
    );

    let output_dir = utils::project_path(project_root, format!("output/{}", arch.output_dir()));
    let images_dir = output_dir.join("images");
    let iso_dir = output_dir.join("iso");

    if !images_dir.exists() {
        anyhow::bail!(
            "No build found at {:?}. Run `man build --arch {}` first.",
            images_dir,
            arch.name()
        );
    }

    // Prepare a staging directory for the ISO
    std::fs::create_dir_all(&iso_dir)?;

    // Copy kernel and rootfs into the ISO staging area
    let kernel_candidates: &[&str] = match arch {
        Arch::Aarch64 => &["Image"],
        Arch::X86_64 => &["bzImage", "vmlinuz"],
    };
    let kernel_src = kernel_candidates
        .iter()
        .map(|name| images_dir.join(name))
        .find(|path| path.exists())
        .unwrap_or_else(|| images_dir.join(kernel_candidates[0]));
    let kernel_name = kernel_src
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(kernel_candidates[0]);
    let initrd_src = images_dir.join("initramfs.cpio.gz");

    if kernel_src.exists() {
        utils::run_command(
            "cp",
            &[
                kernel_src.to_str().unwrap(),
                iso_dir.join(kernel_name).to_str().unwrap(),
            ],
            None,
        )?;
    } else {
        println!("  {} No {} found, skipping", "⚠".yellow(), kernel_name);
    }

    if initrd_src.exists() {
        utils::run_command(
            "cp",
            &[
                initrd_src.to_str().unwrap(),
                iso_dir.join("initrd.img").to_str().unwrap(),
            ],
            None,
        )?;
    } else {
        // Try rootfs.ext2 as initrd
        let rootfs = images_dir.join("rootfs.ext2");
        if rootfs.exists() {
            utils::run_command(
                "cp",
                &[
                    rootfs.to_str().unwrap(),
                    iso_dir.join("initrd.img").to_str().unwrap(),
                ],
                None,
            )?;
        } else {
            println!("  {} No initramfs or rootfs found for ISO", "⚠".yellow());
        }
    }

    let iso_path = output_dir.join(format!("man-{}.iso", arch.name()));

    // Remove existing ISO (hdiutil and mkisofs can't overwrite)
    if iso_path.exists() {
        std::fs::remove_file(&iso_path)?;
    }

    println!("  {} Generating ISO: {}", "→".cyan(), iso_path.display());

    let iso_path_str = iso_path.to_string_lossy().to_string();
    let iso_dir_str = iso_dir.to_string_lossy().to_string();

    // Use mkisofs/genisoimage if available; fall back to macOS hdiutil
    if utils::command_exists("mkisofs") {
        utils::run_command(
            "mkisofs",
            &[
                "-J",
                "-R",
                "-V",
                "MAN",
                "-b",
                "initrd.img",
                "-o",
                &iso_path_str,
                &iso_dir_str,
            ],
            None,
        )?;
    } else if utils::command_exists("genisoimage") {
        utils::run_command(
            "genisoimage",
            &[
                "-J",
                "-R",
                "-V",
                "MAN",
                "-b",
                "initrd.img",
                "-o",
                &iso_path_str,
                &iso_dir_str,
            ],
            None,
        )?;
    } else if utils::command_exists("hdiutil") {
        utils::run_command(
            "hdiutil",
            &[
                "makehybrid",
                "-iso",
                "-joliet",
                "-o",
                &iso_path_str,
                &iso_dir_str,
            ],
            None,
        )?;
    } else {
        anyhow::bail!(
            "No ISO creation tool found. Install mkisofs/genisoimage (e.g. `brew install cdrtools`) or use macOS (hdiutil)."
        );
    }

    let size = std::fs::metadata(&iso_path)?.len();
    println!(
        "\n  {} ISO created: {} ({:.1} MB)",
        "✓".green(),
        iso_path.display(),
        size as f64 / (1024.0 * 1024.0)
    );
    println!(
        "\n  {} Test with: {}",
        "💡".cyan(),
        format!("man test --arch {}", arch.name()).cyan()
    );
    println!();
    Ok(())
}
