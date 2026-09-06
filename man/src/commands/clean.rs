//! `man clean` — remove build artifacts.

use crate::cli::Arch;
use crate::utils;
use clap::Parser;
use colored::*;
use std::path::{Path, PathBuf};

/// Arguments for the `clean` subcommand.
#[derive(Parser, Debug)]
pub struct CleanArgs {
    /// Clean all build artifacts for all architectures.
    #[arg(long)]
    pub all: bool,

    /// Clean only the specified architecture.
    #[arg(short, long, value_enum)]
    pub arch: Option<Arch>,
}

pub fn run(args: &CleanArgs, project_root: &Path) -> anyhow::Result<()> {
    println!("\n{}", "Cleaning MAN build artifacts…".bold().cyan());

    let output_base = utils::project_path(project_root, "output");
    let mut cleaned = false;

    if args.all {
        preserve_isos(&output_base, project_root)?;
        // Clean all architectures
        match std::fs::remove_dir_all(&output_base) {
            Ok(()) => {
                println!("  {} Removed output directory", "✓".green());
                cleaned = true;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                println!("  {} Output directory already clean", "✓".green());
            }
            Err(e) => {
                eprintln!("  {} Error removing output: {}", "✗".red(), e);
            }
        }
    } else if let Some(arch) = args.arch {
        let arch_dir = output_base.join(arch.output_dir());
        preserve_iso(&arch_dir, project_root, arch)?;
        match std::fs::remove_dir_all(&arch_dir) {
            Ok(()) => {
                println!("  {} Removed build for {}", "✓".green(), arch.output_dir());
                cleaned = true;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                println!("  {} No build found for {}", "✓".green(), arch.output_dir());
            }
            Err(e) => {
                eprintln!("  {} Error: {}", "✗".red(), e);
            }
        }
    } else {
        preserve_isos(&output_base, project_root)?;
        // Default: clean all
        match std::fs::remove_dir_all(&output_base) {
            Ok(()) => {
                println!("  {} Removed output directory", "✓".green());
                cleaned = true;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                println!("  {} Output directory already clean", "✓".green());
            }
            Err(e) => {
                eprintln!("  {} Error: {}", "✗".red(), e);
            }
        }
    }

    if cleaned {
        println!("\n  {} Build artifacts cleaned.", "✓".green());
    }
    println!();
    Ok(())
}

/// Copy release ISOs out of disposable build output before cleaning it.
fn preserve_isos(output_base: &Path, project_root: &Path) -> anyhow::Result<()> {
    for arch in [Arch::X86_64, Arch::Aarch64] {
        preserve_iso(&output_base.join(arch.output_dir()), project_root, arch)?;
    }
    Ok(())
}

fn preserve_iso(arch_dir: &Path, project_root: &Path, arch: Arch) -> anyhow::Result<()> {
    let iso_name = match arch {
        Arch::X86_64 => "man-x86-64.iso",
        Arch::Aarch64 => "man-aarch64.iso",
    };
    let source = arch_dir.join(iso_name);
    if !source.is_file() {
        return Ok(());
    }
    let releases = project_root.join("releases").join(arch.output_dir());
    std::fs::create_dir_all(&releases)?;
    let destination: PathBuf = releases.join(iso_name);
    std::fs::copy(&source, &destination)?;
    println!("  {} Preserved release ISO: {}", "✓".green(), destination.display());
    Ok(())
}
