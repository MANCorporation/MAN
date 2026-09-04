//! `man info` — display information about the MAN project.

use crate::utils;
use clap::Parser;
use colored::*;
use std::path::Path;

/// Arguments for the `info` subcommand.
#[derive(Parser, Debug)]
pub struct InfoArgs {
    /// Show detailed build environment information.
    #[arg(long)]
    pub verbose: bool,
}

pub fn run(args: &InfoArgs, project_root: &Path) -> anyhow::Result<()> {
    println!("\n{}", "MAN — A Custom Linux Distribution".bold().cyan());
    println!("{}", "═".repeat(42).dimmed());

    println!("\n  {} NAME", "•".cyan());
    println!("    MAN Linux Distribution");

    println!("\n  {} VERSION", "•".cyan());
    println!("    0.1.0 (alpha)");

    println!("\n  {} ARCHITECTURES", "•".cyan());
    println!("    • x86_64   — standard PCs and servers");
    println!("    • aarch64  — ARM64 (Raspberry Pi, Apple Silicon, ARM servers)");

    println!("\n  {} CORE COMPONENTS", "•".cyan());
    println!("    • Linux kernel (Toolchain-managed)");
    println!("    • musl libc (lightweight C library)");
    println!("    • uutils/coreutils — Rust reimplementation of GNU coreutils");
    println!("    • busybox — rescue shell & additional utilities");
    println!("    • Rust toolchain (rustc + cargo) on the target");

    println!("\n  {} BUILD SYSTEM", "•".cyan());
    println!("    Toolchain (cross-compilation)");

    println!("\n  {} TESTING", "•".cyan());
    println!("    QEMU virtualization (aarch64 + x86_64)");

    if args.verbose {
        println!("\n  {} BUILD ENVIRONMENT", "•".cyan());

        // Check for dependencies
        let deps = [
            ("cargo", "Rust package manager"),
            ("rustc", "Rust compiler"),
            ("make", "GNU Make"),
            ("git", "Git VCS"),
            ("qemu-system-aarch64", "QEMU (aarch64)"),
            ("qemu-system-x86_64", "QEMU (x86_64)"),
        ];

        for (bin, desc) in &deps {
            let status = if utils::command_exists(bin) {
                format!("{} {}", "✓".green(), desc)
            } else {
                format!("{}  {} (not found)", "✗".red(), desc)
            };
            println!("    {:<24}  {}", bin, status);
        }

        let br_dir = utils::project_path(project_root, "toolchain");
        let br_status = if br_dir.join("Makefile").exists() {
            "present".green()
        } else {
            "missing (run `man setup`)".yellow()
        };
        println!("    {:<24}  {}", "toolchain/", br_status);

        // Cargo metadata
        let cargo_toml = utils::project_path(project_root, "man/Cargo.toml");
        if cargo_toml.exists() {
            let content = std::fs::read_to_string(&cargo_toml)?;
            for line in content.lines() {
                if line.starts_with("name") || line.starts_with("version") {
                    println!("    {:<24}  {}", "Cargo.toml", line.trim());
                }
            }
        }
    }

    println!("\n  {} GET STARTED", "•".cyan());
    println!("    {}      — set up build environment", "man setup".cyan());
    println!("    {}   — build the distribution", "man build".cyan());
    println!("    {}      — boot in QEMU for testing", "man test".cyan());
    println!();
    Ok(())
}
