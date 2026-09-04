//! `man config` — display the current MAN configuration.

use crate::utils;
use clap::Parser;
use colored::*;
use std::path::Path;

/// Arguments for the `config` subcommand.
#[derive(Parser, Debug)]
pub struct ConfigArgs {
    /// Print the current MAN configuration.
    #[arg(long)]
    pub print: bool,
}

pub fn run(args: &ConfigArgs, project_root: &Path) -> anyhow::Result<()> {
    println!("\n{}", "MAN Distribution Configuration".bold().cyan());
    println!("{}", "─".repeat(40).dimmed());

    let config = utils::project_path(project_root, "man.toml");

    if config.exists() {
        let content = std::fs::read_to_string(&config)?;
        println!("\n  {} man.toml:", "📄".cyan());
        for line in content.lines() {
            println!("    {}", line);
        }
    } else {
        println!("\n  {} No man.toml found — using defaults", "ℹ".cyan());
    }

    // Show known paths
    let br_dir = utils::project_path(project_root, "toolchain");
    let configs_dir = utils::project_path(project_root, "configs");
    let output_dir = utils::project_path(project_root, "output");

    println!("\n  {} Project paths:", "📁".cyan());
    println!("    root:       {}", project_root.display());
    println!(
        "    toolchain:  {} {}",
        br_dir.display(),
        if br_dir.exists() {
            "(present)".green().to_string()
        } else {
            "(missing — run `man setup`)".yellow().to_string()
        }
    );
    println!("    configs:    {}", configs_dir.display());
    println!("    output:     {}", output_dir.display());

    if args.print {
        println!("\n  {} Full defconfig contents:", "📄".cyan());
        for arch in &["x86_64", "aarch64"] {
            let dc = configs_dir.join(format!("man-{}_defconfig", arch));
            if dc.exists() {
                println!("\n    --- {} ---", dc.display());
                let content = std::fs::read_to_string(&dc)?;
                for line in content.lines() {
                    println!("    {}", line);
                }
            }
        }
    }

    println!();
    Ok(())
}
