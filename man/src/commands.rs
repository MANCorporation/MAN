//! Command modules for the MAN CLI.
//!
//! Each subcommand lives in its own file under `commands/`.
//! Argument structs and `run()` functions are defined per-module
//! and re-exported here for convenience.

pub mod brand_bootloader;
pub mod build;
pub mod clean;
pub mod config;
pub mod cosmic;
pub mod disk;
pub mod info;
pub mod iso;
pub mod release;
pub mod setup;
pub mod test;

// Re-export argument types so main.rs can import them cleanly.
pub use brand_bootloader::BrandBootloaderArgs;
pub use build::BuildArgs;
pub use clean::CleanArgs;
pub use config::ConfigArgs;
pub use cosmic::CosmicArgs;
pub use disk::DiskArgs;
pub use info::InfoArgs;
pub use iso::IsoArgs;
pub use release::ReleaseArgs;
pub use setup::SetupArgs;
pub use test::TestArgs;
