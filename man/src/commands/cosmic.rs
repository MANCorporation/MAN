//! `man cosmic` — build and install the MAN Desktop (rebranded COSMIC) from source.
//!
//! COSMIC (pop-os/cosmic-epoch) is a Rust + Wayland desktop. MAN Desktop is a
//! rebranded build of COSMIC. Because COSMIC is not available as a Buildroot
//! package, this subcommand delegates to `scripts/build-cosmic.sh` which
//! cross-compiles each COSMIC crate with cargo and installs the binaries +
//! shared libraries into the rootfs staging directory.

use crate::cli::Arch;
use crate::utils;
use clap::Parser;
use colored::Colorize;
use indicatif::ProgressBar;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

/// Arguments for the `cosmic` subcommand.
#[derive(Parser, Debug)]
pub struct CosmicArgs {
    /// Target architecture.
    #[arg(short, long, value_enum, default_value = "aarch64")]
    pub arch: Arch,

    /// Remove the COSMIC build cache before building.
    #[arg(long)]
    pub clean: bool,

    /// COSMIC Epoch release tag to build.
    #[arg(long, default_value = "epoch-1.6.0")]
    pub version: String,

    /// Install into the staging rootfs without regenerating boot images.
    #[arg(long)]
    pub no_image: bool,
}

/// Result returned to main.rs after running the Cosmic subcommand.
pub fn run(args: &CosmicArgs, project_root: &Path) -> anyhow::Result<()> {
    let arch = args.arch;
    println!(
        "\n{}",
        format!("Building MAN Desktop (COSMIC) for {}…", arch.name())
            .bold()
            .cyan()
    );

    let script = utils::project_path(project_root, "scripts/build-cosmic.sh");
    if !script.exists() {
        anyhow::bail!("COSMIC build script not found at {:?}.", script);
    }

    if args.clean {
        let build_dir = utils::project_path(
            project_root,
            format!("output/{}/build/cosmic-epoch", arch.output_dir()),
        );
        println!("  {} Cleaning previous COSMIC build…", "→".cyan());
        let _ = std::fs::remove_dir_all(&build_dir);
    }

    let bar = ProgressBar::new(if args.no_image { 3 } else { 4 });
    let start = Instant::now();

    // Step 1 — clone / update source
    bar.set_message("Downloading COSMIC source");
    bar.inc(1);

    // Step 2 + 3 — cross-compile (delegated to the shell script)
    bar.set_message("Compiling COSMIC crates (cross)");
    let mut cosmic_env = HashMap::new();
    cosmic_env.insert("COSMIC_VERSION".to_string(), args.version.clone());
    utils::run_command_with_env(
        "bash",
        &[script.to_str().unwrap(), arch.name()],
        Some(project_root),
        Some(cosmic_env),
        None,
    )?;
    bar.inc(1);

    // Step 4 — verify install
    bar.set_message("Verifying installation");
    let target_dir =
        utils::project_path(project_root, format!("output/{}/target", arch.output_dir()));
    let bin = target_dir.join("usr/bin/cosmic-comp");
    if bin.exists() {
        println!("  {} Installed: {}", "✓".green(), bin.display());
        bar.inc(1);
    } else {
        bar.finish_and_clear();
        anyhow::bail!("COSMIC build finished without installing cosmic-comp");
    }

    if !args.no_image {
        bar.set_message("Regenerating MAN boot images");
        let output_dir = utils::project_path(project_root, format!("output/{}", arch.output_dir()));
        let toolchain_dir = utils::project_path(project_root, "toolchain");
        let output_arg = format!("O={}", output_dir.display());
        let make = if cfg!(target_os = "macos") && utils::command_exists("gmake") {
            "gmake"
        } else {
            "make"
        };
        let env = if cfg!(target_os = "macos") {
            Some(utils::setup_macos_env())
        } else {
            None
        };
        utils::run_command_with_env(
            make,
            &[&output_arg],
            Some(&toolchain_dir),
            env,
            Some(utils::UNSET_ENV_VARS.to_vec()),
        )?;
        bar.inc(1);
    }

    bar.finish_and_clear();

    let elapsed = start.elapsed();
    println!(
        "\n  {} MAN Desktop build completed in {:.1}s",
        "✓".green(),
        elapsed.as_secs_f64()
    );
    println!("  {} Test with: man test --arch {}", "💡".cyan(), arch.name());
    println!();
    Ok(())
}
