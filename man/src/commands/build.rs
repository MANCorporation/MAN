//! `man build` — build the MAN distribution via Toolchain.
//!
//! On macOS, Toolchain can build natively using GNU Make (gmake) with
//! a macOS-compatible toolchain shim (gnu-shim) that wraps GCC, autoconf,
//! and other GNU tools. The CLI sets up the proper PATH, SSL certificate
//! paths, and unsets PERL5OPT/PERL5LIB so that the glibc cross-compile
//! build succeeds. Docker is used as a fallback when gmake is not available.

use crate::cli::Arch;
use crate::utils;
use clap::Parser;
use colored::*;
use std::path::Path;
use std::time::Instant;

/// Arguments for the `build` subcommand.
#[derive(Parser, Debug)]
pub struct BuildArgs {
    /// Target architecture.
    #[arg(short, long, value_enum, default_value = "x86_64")]
    pub arch: Arch,

    /// Number of parallel make jobs (default: number of CPUs).
    #[arg(short, long)]
    pub jobs: Option<usize>,

    /// Clean before building.
    #[arg(long)]
    pub clean: bool,

    /// Force Docker build even on Linux.
    #[arg(long)]
    pub docker: bool,
}

pub fn run(args: &BuildArgs, project_root: &Path) -> anyhow::Result<()> {
    let arch = args.arch;
    println!("\n{}", format!("Building MAN for {}…", arch.name()).bold().cyan());

    let br_dir = utils::project_path(project_root, "toolchain");
    let output_dir = utils::project_path(project_root, format!("output/{}", arch.output_dir()));
    let project_defconfig =
        utils::project_path(project_root, format!("configs/{}_defconfig", arch.defconfig_name()));
    let toolchain_defconfig = br_dir
        .join("configs")
        .join(format!("{}_defconfig", arch.defconfig_name()));

    if !br_dir.join("Makefile").exists() {
        anyhow::bail!("Toolchain not found at {:?}. Run `man setup` first.", br_dir);
    }
    if !project_defconfig.exists() {
        anyhow::bail!("MAN defconfig not found at {:?}", project_defconfig);
    }

    // The project copy is authoritative. Keeping Buildroot's copied defconfig
    // synchronized makes new dependencies take effect on every build, not only
    // after the setup command.
    std::fs::copy(&project_defconfig, &toolchain_defconfig)?;

    // Keep project-owned Buildroot compatibility patches available when the
    // Toolchain checkout is bind-mounted over the Docker image's /MAN tree.
    let mpv_patch = utils::project_path(
        project_root,
        "patches/buildroot/mpv/0002-fix-python-3.12-argparse-type.patch",
    );
    if mpv_patch.exists() {
        std::fs::copy(
            &mpv_patch,
            br_dir
                .join("package/mpv")
                .join(mpv_patch.file_name().unwrap()),
        )?;
    }

    // Optionally clean
    if args.clean {
        println!("  {} Cleaning previous build output", "→".cyan());
        let _ = std::fs::remove_dir_all(&output_dir);
    }

    std::fs::create_dir_all(&output_dir)?;

    // Determine parallel jobs
    let jobs = args.jobs.unwrap_or_else(num_cpus);
    println!("  {} Using {} parallel jobs", "→".cyan(), jobs);

    let start = Instant::now();
    let out_str = output_dir.to_str().unwrap();
    let br_str = br_dir.to_str().unwrap();
    let defconfig = format!("{}_defconfig", arch.defconfig_name());

    // Detect platform and choose build method
    let is_macos = cfg!(target_os = "macos");
    let make_cmd = if is_macos && utils::command_exists("gmake") {
        "gmake"
    } else if utils::command_exists("make") {
        "make"
    } else {
        "make" // let it fail with a clear error
    };

    // On macOS, Toolchain can build natively if GNU Make 4+ (gmake) is available.
    // Docker is used as a fallback or when explicitly requested.
    let using_docker = args.docker
        || (is_macos && !utils::command_exists("gmake") && utils::command_exists("docker"));

    if using_docker {
        println!("  {} macOS detected — using Docker for Linux build", "🐳".cyan());
        // Docker mounts the project root at /MAN, so its output argument must
        // be project-relative. `out_str` is an absolute host path.
        let docker_out_dir = format!("output/{}", arch.output_dir());
        run_docker_build(project_root, arch, jobs, &docker_out_dir, &defconfig)?;
    } else if is_macos {
        println!(
            "  {} macOS detected — using {} with macOS cross-compile env",
            "🐳".cyan(),
            make_cmd
        );
        let macos_env = utils::setup_macos_env();
        // Run defconfig first (required before building — same as Docker path)
        println!("  {} Running Toolchain {} {}", "→".cyan(), make_cmd, defconfig.cyan());
        utils::run_command_with_env(
            make_cmd,
            &[&format!("O={}", out_str), &defconfig],
            Some(Path::new(br_str)),
            Some(macos_env.clone()),
            Some(utils::UNSET_ENV_VARS.to_vec()),
        )?;
        println!("  {} Running Toolchain {} (this may take a while…)", "→".cyan(), make_cmd);
        utils::run_command_with_env(
            make_cmd,
            &[&format!("O={}", out_str), &format!("-j{}", jobs)],
            Some(Path::new(br_str)),
            Some(macos_env),
            Some(utils::UNSET_ENV_VARS.to_vec()),
        )?;
    } else {
        println!("  {} Running Toolchain make (this may take a while…)", "→".cyan());
        // Run defconfig first (required before building — same as Docker path)
        println!("  {} Running Toolchain make {}", "→".cyan(), defconfig.cyan());
        utils::run_command(
            "make",
            &[&format!("O={}", out_str), &defconfig],
            Some(Path::new(br_str)),
        )?;
        utils::run_command(
            "make",
            &[&format!("O={}", out_str), &format!("-j{}", jobs)],
            Some(Path::new(br_str)),
        )?;
    }

    let elapsed = start.elapsed();
    println!("\n  {} Build completed in {:.1}s", "✓".green(), elapsed.as_secs_f64());

    // Locate and report the output image
    let images_dir = output_dir.join("images");
    if images_dir.exists() {
        println!("\n  {} Output images:", "📦".cyan());
        for entry in std::fs::read_dir(&images_dir)? {
            let entry = entry?;
            let name = entry.file_name();
            println!("    {}", name.to_string_lossy().cyan());
        }
    }

    println!(
        "\n  {} Test with: {}",
        "💡".cyan(),
        format!("man test --arch {}", arch.name()).cyan()
    );
    println!();
    Ok(())
}

fn run_docker_build(
    project_root: &Path,
    arch: Arch,
    jobs: usize,
    out_str: &str,
    defconfig: &str,
) -> anyhow::Result<()> {
    if !utils::command_exists("docker") {
        anyhow::bail!(
            "Docker is required for building on macOS. Please install Docker Desktop or Docker Engine.
            Alternatively, build on a Linux machine or VM.",
        );
    }

    let image_name = format!("man-builder:ci");
    println!("  {} Using Docker build environment", "🐳".cyan());

    // Build the Docker image if it doesn't exist
    let dockerfile = utils::project_path(project_root, "Dockerfile");
    if !docker_cmd_has_image(&image_name) {
        println!("  {} Building Docker image (first time only)…", "→".cyan());
        utils::run_command(
            "docker",
            &[
                "build",
                "-t",
                &image_name,
                "-f",
                dockerfile.to_str().unwrap(),
                ".",
            ],
            Some(project_root),
        )?;
    }

    // Run Toolchain build inside Docker
    let project_str = project_root.to_str().unwrap();
    println!("  {} Running Toolchain make in Docker container…", "→".cyan());
    println!("  {} Architecture: {} | Jobs: {}", "→".cyan(), arch.name(), jobs);

    utils::run_command(
        "docker",
        &[
            "run",
            "--rm",
            "-v",
            &format!("{}:/MAN", project_str),
            "-w",
            "/MAN/toolchain",
            &image_name,
            "sh",
            "-c",
            &format!(
                "make O=/MAN/{0} {1} && \
                 sed -i 's|^BR2_LINUX_KERNEL_PATCH=.*|BR2_LINUX_KERNEL_PATCH=\"\"|' /MAN/{0}/.config && \
                 make O=/MAN/{0} olddefconfig && \
                 make O=/MAN/{0} -j{2}",
                out_str, defconfig, jobs
            ),
        ],
        None,
    )?;

    println!("  {} Docker build complete", "✓".green());
    Ok(())
}

fn docker_cmd_has_image(image: &str) -> bool {
    utils::capture_command("docker", &["inspect", "--type", "image", image], None)
        .map(|s| !s.contains("No such image"))
        .unwrap_or(false)
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
