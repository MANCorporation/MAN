//! In-place MAN branding for the bundled EFI boot-manager executables.

use anyhow::{bail, Context, Result};
use clap::Parser;
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
pub struct BrandBootloaderArgs {
    /// EFI executables to brand. Defaults to the bundled ARM64 and x86-64 files.
    #[arg(value_name = "EFI")]
    files: Vec<PathBuf>,
}

fn utf16le(value: &str) -> Vec<u8> {
    value.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

fn replace_equal_width(data: &mut [u8], old: &str, new: &str) -> Result<usize> {
    let old = utf16le(old);
    let new = utf16le(new);
    if old.len() != new.len() {
        bail!("EFI string replacements must have equal UTF-16 width");
    }

    let mut count = 0;
    let mut offset = 0;
    while let Some(relative) = data[offset..]
        .windows(old.len())
        .position(|candidate| candidate == old)
    {
        let start = offset + relative;
        data[start..start + new.len()].copy_from_slice(&new);
        count += 1;
        offset = start + new.len();
    }
    Ok(count)
}

fn brand(path: &Path) -> Result<usize> {
    let mut data = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    if !data.starts_with(b"MZ") {
        bail!("{} is not a PE/COFF EFI executable", path.display());
    }

    // These are deliberately equal-width substitutions: no PE section moves,
    // offsets, or executable code are changed. Lowercase `refind` is replaced
    // too, so private paths and diagnostics use man-os.conf/man-os-vars.
    let replacements = [
        ("rEFInd", "MAN OS"),
        ("REFIND", "MAN OS"),
        ("Refind", "MAN OS"),
        ("refind", "man-os"),
    ];
    let mut changed = 0;
    for (old, new) in replacements {
        changed += replace_equal_width(&mut data, old, new)?;
    }

    if changed > 0 {
        fs::write(path, data).with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(changed)
}

pub fn run(args: &BrandBootloaderArgs, project_root: &Path) -> Result<()> {
    let files = if args.files.is_empty() {
        vec![
            project_root.join("scripts/bootloader/BOOTAA64.EFI"),
            project_root.join("scripts/bootloader/BOOTX64.EFI"),
        ]
    } else {
        args.files
            .iter()
            .map(|path| {
                if path.is_absolute() {
                    path.clone()
                } else {
                    project_root.join(path)
                }
            })
            .collect()
    };

    for file in files {
        let changed = brand(&file)?;
        if changed == 0 {
            println!("  {} {} is already MAN-branded", "✓".green(), file.display());
        } else {
            println!(
                "  {} replaced {changed} EFI UI/path strings in {}",
                "✓".green(),
                file.display()
            );
        }
    }
    Ok(())
}
