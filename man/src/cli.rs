//! Project-wide constants and the Arch enum.

use clap::ValueEnum;

/// The architectures MAN supports.
#[derive(Copy, Clone, PartialEq, Eq, Debug, ValueEnum)]
pub enum Arch {
    /// x86_64 (AMD64) — for standard PCs and most servers.
    X86_64,
    /// ARM64 / AArch64 — for Raspberry Pi, Apple Silicon, and modern ARM servers.
    Aarch64,
}

impl Arch {
    /// Human-readable name used in Toolchain output directories.
    pub fn output_dir(&self) -> &'static str {
        match self {
            Arch::X86_64 => "x86_64",
            Arch::Aarch64 => "aarch64",
        }
    }

    /// The Toolchain defconfig name (without `_defconfig`).
    pub fn defconfig_name(&self) -> &'static str {
        match self {
            Arch::X86_64 => "man-x86_64",
            Arch::Aarch64 => "man-aarch64",
        }
    }

    /// The QEMU machine name used for this architecture.
    pub fn qemu_machine(&self) -> &'static str {
        match self {
            Arch::X86_64 => "q35",
            Arch::Aarch64 => "virt",
        }
    }

    /// The QEMU binary to use for this architecture.
    pub fn qemu_binary(&self) -> &'static str {
        match self {
            Arch::X86_64 => "qemu-system-x86_64",
            Arch::Aarch64 => "qemu-system-aarch64",
        }
    }

    /// File-system image name produced by Toolchain.
    #[allow(dead_code)]
    pub fn fs_image(&self) -> &'static str {
        match self {
            Arch::X86_64 => "sdcard.img",
            Arch::Aarch64 => "sdcard.img",
        }
    }

    /// Short architecture name for display and logging.
    pub fn name(&self) -> &'static str {
        match self {
            Arch::X86_64 => "x86_64",
            Arch::Aarch64 => "aarch64",
        }
    }

    /// QEMU EFI firmware path for this architecture (Homebrew).
    pub fn qemu_firmware(&self) -> &'static str {
        match self {
            Arch::X86_64 => "/Users/kristihack/.homebrew/share/qemu/edk2-x86_64-code.fd",
            Arch::Aarch64 => "/Users/kristihack/.homebrew/share/qemu/edk2-aarch64-code.fd",
        }
    }
}
