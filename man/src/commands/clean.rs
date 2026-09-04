//! `man clean` — remove build artifacts.

use crate::cli::Arch;
use crate::utils;
use clap::Parser;
use colored::*;
use std::path::Path;

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
