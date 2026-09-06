//! `man release` — build and package ISO images for all supported targets.

use crate::cli::Arch;
use crate::commands::{build, iso};
use clap::Parser;
use colored::*;
use std::path::Path;
use std::time::Instant;

/// Arguments for the `release` subcommand.
#[derive(Parser, Debug)]
pub struct ReleaseArgs {
    /// Number of parallel make jobs per architecture (default: number of CPUs).
    #[arg(short, long)]
    pub jobs: Option<usize>,

    /// Clean each architecture's build directory before building.
    #[arg(long)]
    pub clean: bool,

    /// Force Docker builds where supported.
    #[arg(long)]
    pub docker: bool,
}

/// Build sequentially so each target uses all available CPU and memory. The
/// Buildroot download directory and compiler cache are shared by both targets.
pub fn run(args: &ReleaseArgs, project_root: &Path) -> anyhow::Result<()> {
    println!(
        "\n{}",
        "Building MAN release ISOs for x86_64 and aarch64…"
            .bold()
            .cyan()
    );
    let start = Instant::now();

    for arch in [Arch::X86_64, Arch::Aarch64] {
        build::run(
            &build::BuildArgs {
                arch,
                jobs: args.jobs,
                clean: args.clean,
                docker: args.docker,
            },
            project_root,
        )?;
        iso::run(&iso::IsoArgs { arch }, project_root)?;
    }

    println!(
        "\n  {} Release ISOs completed in {:.1}s",
        "✓".green(),
        start.elapsed().as_secs_f64()
    );
    println!("    output/x86_64/man-x86-64.iso");
    println!("    output/aarch64/man-aarch64.iso\n");
    Ok(())
}
