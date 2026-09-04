//! manctl — MAN Linux Distribution Control & Info Utility.
//!
//! This tool ships inside the MAN distribution and provides system
//! information, build introspection, and package management helpers.
//!
//! Run `manctl` with no arguments for a summary, or use subcommands:
//!   manctl version  — show MAN version & build info
//!   manctl info     — show system information
//!   manctl packages — list installed packages
//!   manctl help     — show this help

use std::env;
use std::fs;
use std::process;

// Simple ANSI color helpers (no external dependencies — keeps the binary tiny)
#[allow(dead_code)]
mod color {
    pub const CYAN: &str = "\x1b[0;36m";
    pub const GREEN: &str = "\x1b[0;32m";
    pub const YELLOW: &str = "\x1b[0;33m";
    pub const BOLD: &str = "\x1b[1m";
    pub const RESET: &str = "\x1b[0m";

    pub fn cyan(s: &str) -> String { format!("{}{}{}", CYAN, s, RESET) }
    pub fn green(s: &str) -> String { format!("{}{}{}", GREEN, s, RESET) }
    pub fn yellow(s: &str) -> String { format!("{}{}{}", YELLOW, s, RESET) }
    pub fn bold(s: &str) -> String { format!("{}{}{}", BOLD, s, RESET) }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    match args.len() {
        1 => print_summary(),
        _ => match args[1].as_str() {
            "version" => print_version(),
            "info" => print_info(),
            "packages" => print_packages(),
            "help" | "--help" | "-h" => print_help(),
            other => {
                eprintln!("Unknown command: {}", other);
                print_help();
                process::exit(1);
            }
        },
    }
}

fn print_summary() {
    println!("╔══════════════════════════════════════════════╗");
    println!("║          MAN Linux Distribution              ║");
    println!("║            v0.1.0 (alpha)                     ║");
    println!("╚══════════════════════════════════════════════╝");
    println!();
    println!("  Run 'manctl help' for available commands.");
    println!();

    // Show a brief OS summary
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                if matches!(key, "NAME" | "VERSION" | "PRETTY_NAME") {
                    println!("  {}: {}", key.to_lowercase(), value.trim_matches('"'));
                }
            }
        }
    }
}

fn print_version() {
    println!("{}", color::bold("MAN Linux Distribution"));
    println!("  Version: 0.1.0 (alpha)");
    println!("  Build ID: alpha");
    println!("  Architectures: x86_64, aarch64");

    if let Ok(content) = fs::read_to_string("/proc/version") {
        println!();
        println!("  Kernel: {}", content.lines().next().unwrap_or("unknown"));
    }

    if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
        for line in content.lines() {
            if let Some((key, value)) = line.split_once(':') {
                if key.trim() == "model name" || key.trim() == "Processor" {
                    println!("  CPU:  {}", value.trim());
                    break;
                }
            }
        }
    }

    if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                println!("  RAM:  {}", line.split(':').nth(1).unwrap_or("unknown").trim());
                break;
            }
        }
    }
}

fn print_info() {
    println!("{}", color::bold("MAN System Information"));
    println!("{}", "─".repeat(40));
    println!();

    // Read /etc/os-release
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        println!("{}:", color::cyan("OS Release"));
        for line in content.lines() {
            if !line.is_empty() {
                println!("  {}", line);
            }
        }
    }

    // Read /proc/version
    if let Ok(content) = fs::read_to_string("/proc/version") {
        println!();
        println!("{}: {}", color::cyan("Kernel"), content.lines().next().unwrap_or(""));
    }

    // Read /proc/cpuinfo (first processor)
    if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
        let mut shown = false;
        println!();
        println!("{}", color::cyan("CPU:"));
        for line in content.lines() {
            if line.starts_with("processor") && shown {
                break;
            }
            if !line.is_empty() && !line.starts_with("processor") {
                println!("  {}", line);
            }
            if line.starts_with("processor") {
                shown = true;
            }
        }
    }

    // Read /proc/meminfo
    if let Ok(content) = fs::read_to_string("/proc/meminfo") {
        println!();
        println!("{}", color::cyan("Memory:"));
        for line in content.lines().take(5) {
            println!("  {}", line);
        }
    }

    // Disk usage
    if let Ok(entries) = fs::read_dir("/") {
        let count = entries.flatten().count();
        println!();
        println!("{}", color::cyan("Filesystem:"));
        println!("  Root partition contains {} entries", count);
    }
}

fn print_packages() {
    println!("{}", color::bold("MAN Installed Packages"));
    println!("{}", "─".repeat(40));
    println!();

    println!("  uutils/coreutils  — Rust reimplementation of GNU coreutils");
    println!("  busybox           — rescue shell & utilities");
    println!("  rustc + cargo     — Rust toolchain");
    println!("  bash + zsh        — login shells");
    println!("  openssh           — SSH server/client");

    // Try to locate /var/lib/pkg or similar
    if let Ok(entries) = fs::read_dir("/usr/lib/pkg") {
        let mut packages: Vec<String> = Vec::new();
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                packages.push(name.to_string());
            }
        }
        packages.sort();
        for pkg in packages {
            println!("  {}{}", color::green(""), pkg);
        }
    }
}

fn print_help() {
    println!("{}", color::bold("manctl — MAN Linux Distribution Control Tool"));
    println!();
    println!("USAGE: manctl [COMMAND]");
    println!();
    println!("COMMANDS:");
    println!("  (none)    Show distribution summary");
    println!("  version   Show version & system details");
    println!("  info      Show detailed system information");
    println!("  packages  List installed packages");
    println!("  help      Show this help message");
    println!();
    println!("EXAMPLES:");
    println!("  manctl version");
    println!("  manctl info");
}
