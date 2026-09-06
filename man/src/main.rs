//! MAN — a custom Linux distribution orchestrator.
//!
//! This CLI manages building, testing, and packaging the MAN Linux
//! distribution. It wraps Toolchain, QEMU, and mkisofs to provide a
//! single entry point for both x86_64 and aarch64 targets.

use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;

mod cli;
mod commands;
mod utils;

use commands::{
    BrandBootloaderArgs, BuildArgs, CleanArgs, ConfigArgs, CosmicArgs, DiskArgs, InfoArgs, IsoArgs,
    ReleaseArgs, SetupArgs, TestArgs,
};

#[derive(Parser)]
#[command(
    name = "man",
    bin_name = "man",
    version,
    about = "Orchestrates building, testing, and packaging the MAN Linux distribution"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Replace upstream-facing EFI UI strings with MAN branding.
    BrandBootloader(BrandBootloaderArgs),

    /// Set up the build environment: install dependencies, clone Toolchain.
    Setup(SetupArgs),

    /// Build the MAN distribution for one or both architectures.
    Build(BuildArgs),

    /// Boot the built image in QEMU for testing.
    Test(TestArgs),

    /// Create an ISO image from the built root filesystem.
    Iso(IsoArgs),

    /// Build and package bootable ISO images for every supported architecture.
    Release(ReleaseArgs),

    /// Create a UEFI-bootable disk image from the built rootfs.
    Disk(DiskArgs),

    /// Show or edit the MAN configuration.
    Config(ConfigArgs),

    /// Clean build artifacts.
    Clean(CleanArgs),

    /// Build and install the MAN Desktop (rebranded COSMIC) from source.
    Cosmic(CosmicArgs),

    /// Show information about the MAN project.
    Info(InfoArgs),
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Ensure we're running from the project root (or a known location).
    let project_root = utils::find_project_root().unwrap_or_else(|| PathBuf::from("."));

    println!();
    println!("{}", "  M A N  —  Linux Distribution Orchestrator".bold().cyan());
    println!("{}", "  version".dimmed());

    match &cli.command {
        Commands::BrandBootloader(args) => commands::brand_bootloader::run(args, &project_root),
        Commands::Setup(args) => commands::setup::run(args, &project_root),
        Commands::Build(args) => commands::build::run(args, &project_root),
        Commands::Test(args) => commands::test::run(args, &project_root),
        Commands::Iso(args) => commands::iso::run(args, &project_root),
        Commands::Release(args) => commands::release::run(args, &project_root),
        Commands::Disk(args) => commands::disk::run(args, &project_root),
        Commands::Config(args) => commands::config::run(args, &project_root),
        Commands::Clean(args) => commands::clean::run(args, &project_root),
        Commands::Cosmic(args) => commands::cosmic::run(args, &project_root),
        Commands::Info(args) => commands::info::run(args, &project_root),
    }
}
