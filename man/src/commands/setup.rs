//! `man setup` — prepare the build environment.

use crate::cli::Arch;
use crate::utils;
use clap::Parser;
use colored::*;
use std::path::Path;

/// Arguments for the `setup` subcommand.
#[derive(Parser, Debug)]
pub struct SetupArgs {
    /// Skip cloning the Toolchain source tree (use existing clone).
    #[arg(long)]
    pub no_clone: bool,

    /// Install system dependencies via Homebrew.
    #[arg(long)]
    pub install_deps: bool,

    /// Also set up the aarch64 target (in addition to x86_64).
    #[arg(long)]
    pub all_archs: bool,
}

const TOOLCHAIN_REPO: &str = "https://github.com/buildroot/buildroot.git";
const TOOLCHAIN_TAG: &str = "2024.08"; // Stable release tag

pub fn run(args: &SetupArgs, project_root: &Path) -> anyhow::Result<()> {
    println!("\n{}", "Setting up MAN build environment…".bold().cyan());

    if args.install_deps {
        install_deps()?;
    }

    if !args.no_clone {
        clone_toolchain(project_root)?;
    }

    // Apply MAN defconfigs
    let archs = if args.all_archs {
        vec![Arch::X86_64, Arch::Aarch64]
    } else {
        vec![Arch::X86_64]
    };

    for arch in archs {
        apply_defconfig(project_root, arch)?;
    }

    println!("\n{}", "✓ Setup complete!".green().bold());
    println!();
    println!("  Next steps:");
    println!("    {}  — build the distribution", "man build".cyan());
    println!("    {}  — boot in QEMU to test", "man test".cyan());
    println!();
    Ok(())
}

fn install_deps() -> anyhow::Result<()> {
    println!("\n  {} System dependencies", "→".cyan());

    // Locate Homebrew — check common paths since it may be at a non-standard prefix.
    let home = std::env::var("HOME").unwrap_or_default();
    let homebrew_paths: [String; 3] = [
        format!("{}/.homebrew/bin", home),
        "/opt/homebrew/bin".to_string(),
        "/usr/local/bin".to_string(),
    ];

    let brew_bin = homebrew_paths
        .iter()
        .find(|p| Path::new(p).join("brew").exists())
        .map(|p| Path::new(p).join("brew"))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Homebrew not found. Install it via https://brew.sh \
                 or run: git clone --depth 1 https://github.com/Homebrew/brew.git ~/.homebrew"
            )
        })?;

    println!("  {} Using Homebrew at {:?}", "✓".green(), brew_bin);

    // Install QEMU for testing, plus core build utilities.
    // mtools is needed for creating UEFI disk images (`man disk`).
    let deps = [
        "qemu",
        "coreutils",
        "make",
        "gcc",
        "gperf",
        "unzip",
        "rsync",
        "flex",
        "bison",
        "libelf",
        "mtools",
        "xorriso",
    ];

    // Run brew with its bin directory prepended to PATH
    let brew_dir = brew_bin.parent().unwrap();
    let current_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", brew_dir.display(), current_path);

    let mut cmd = std::process::Command::new(&brew_bin);
    cmd.arg("install");
    for dep in &deps {
        cmd.arg(dep);
    }
    cmd.env("PATH", &new_path);

    let status = cmd.status()?;
    if !status.success() {
        anyhow::bail!("brew install failed with exit code {:?}", status.code());
    }

    println!("  {} Dependencies installed", "✓".green());
    Ok(())
}

fn clone_toolchain(project_root: &Path) -> anyhow::Result<()> {
    let br_dir = utils::project_path(project_root, "toolchain");

    if br_dir.join(".git").exists() {
        println!("  {} Toolchain already present at {:?}", "✓".green(), br_dir);
        // Update to the pinned tag
        println!("  {} Checking out tag {}", "→".cyan(), TOOLCHAIN_TAG);
        utils::run_command("git", &["-C", br_dir.to_str().unwrap(), "fetch", "--tags"], None)?;
        utils::run_command(
            "git",
            &["-C", br_dir.to_str().unwrap(), "checkout", TOOLCHAIN_TAG],
            None,
        )?;
    } else {
        println!("  {} Cloning Toolchain {} (tag {})", "→".cyan(), TOOLCHAIN_REPO, TOOLCHAIN_TAG);
        utils::run_command(
            "git",
            &[
                "clone",
                "--depth",
                "1",
                "--branch",
                TOOLCHAIN_TAG,
                TOOLCHAIN_REPO,
                br_dir.to_str().unwrap(),
            ],
            None,
        )?;
    }

    println!("  {} Toolchain ready", "✓".green());
    Ok(())
}

fn apply_defconfig(project_root: &Path, arch: Arch) -> anyhow::Result<()> {
    let br_dir = utils::project_path(project_root, "toolchain");
    let configs_dir = utils::project_path(project_root, "configs");
    let output_dir = utils::project_path(project_root, format!("output/{}", arch.output_dir()));

    let defconfig = format!("{}_defconfig", arch.defconfig_name());
    let defconfig_path = configs_dir.join(&defconfig);

    if !defconfig_path.exists() {
        anyhow::bail!(
            "Defconfig not found at {:?}. Run with appropriate config creation.",
            defconfig_path
        );
    }

    println!("\n  {} Configuring Toolchain for {}", "→".cyan(), arch.defconfig_name());
    std::fs::create_dir_all(&output_dir)?;

    // Copy our defconfig into Toolchain's configs directory
    let dest = br_dir.join("configs").join(&defconfig);
    utils::run_command("cp", &[defconfig_path.to_str().unwrap(), dest.to_str().unwrap()], None)?;

    // Run make with O=output_dir to configure
    // On macOS, use gmake (GNU Make 4+) if available
    let make_cmd = if cfg!(target_os = "macos") && utils::command_exists("gmake") {
        "gmake"
    } else {
        "make"
    };
    let arch_str = arch.output_dir();
    let br_str = br_dir.to_str().unwrap();
    let out_str = output_dir.to_str().unwrap();
    utils::run_command(
        make_cmd,
        &[&format!("O={}", out_str), &defconfig],
        Some(Path::new(br_str)),
    )?;

    println!("  {} Toolchain configured for {}", "✓".green(), arch_str);
    Ok(())
}
