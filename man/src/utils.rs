//! Utility functions used across the CLI.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Check whether a binary is available on PATH.
pub fn command_exists(cmd: &str) -> bool {
    which::which(cmd).is_ok()
}

/// Walk up from the current directory to find the project root.
/// The project root is the directory that contains `man/Cargo.toml`.
pub fn find_project_root() -> Option<PathBuf> {
    let start = std::env::current_dir().ok()?;
    let mut current: &Path = &start;
    loop {
        if current.join("man").join("Cargo.toml").exists() {
            return Some(current.to_path_buf());
        }
        current = current.parent()?;
    }
}

/// Resolve a path relative to the project root.
pub fn project_path<P: AsRef<Path>>(root: &Path, parts: P) -> PathBuf {
    root.join(parts)
}

/// Run a shell command and stream its output.
pub fn run_command(cmd: &str, args: &[&str], cwd: Option<&Path>) -> anyhow::Result<()> {
    run_command_with_env(cmd, args, cwd, None, None)
}

/// Run a shell command with optional environment variables.
///
/// `env` — key/value pairs to set in the child process.
/// `unset_env` — variable names to remove from the child's environment
///   (useful on macOS to strip inherited Perl configuration before Toolchain).
pub fn run_command_with_env(
    cmd: &str,
    args: &[&str],
    cwd: Option<&Path>,
    env: Option<std::collections::HashMap<String, String>>,
    unset_env: Option<Vec<&str>>,
) -> anyhow::Result<()> {
    let mut command = Command::new(cmd);
    command.args(args);
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }
    if let Some(unset_vars) = unset_env {
        for key in unset_vars {
            command.env_remove(key);
        }
    }
    if let Some(env_vars) = env {
        for (key, value) in env_vars {
            command.env(key, value);
        }
    }
    let status = command.status()?;
    if !status.success() {
        anyhow::bail!("command {:?} failed with exit code {:?}", status.code(), cmd);
    }
    Ok(())
}

/// Build the environment HashMap needed for macOS cross-compilation builds.
///
/// Sets up the GNU tools shim PATH, SSL certificate paths, and `HOME`
/// so that Toolchain's glibc build can find `gmake`, GNU autoconf tools,
/// and the Perl `osx_fix` module. Callers should also unset inherited
/// `PERL5LIB` (see [`UNSET_ENV_VARS]).
pub fn setup_macos_env() -> std::collections::HashMap<String, String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let gnu_shim = format!("{}/.homebrew/gnu-shim", home);
    let homebrew_bin = format!("{}/.homebrew/bin", home);
    let cargo_bin = format!("{}/.cargo/bin", home);
    let local_bin = format!("{}/.local/bin", home);
    let ssl_cert = format!("{}/.homebrew/etc/ca-certificates/cert.pem", home);
    let libelf_include = format!(
        "{}/.homebrew/opt/libelf/include:{}/.homebrew/opt/libelf/include/libelf",
        home, home
    );

    // GNU shims first (so gcc, ar, ld etc. are the macOS-compatible wrappers),
    // then Homebrew GNU tools, then system paths, then cargo/local bin.
    // GNU shims first, then user Homebrew, then system Homebrew (for mtools etc.),
    // then system paths, then cargo/local bin.
    let clean_path = format!(
        "{}:{}:/opt/homebrew/bin:/opt/homebrew/sbin:/usr/bin:/bin:/usr/sbin:/sbin:{}:{}",
        gnu_shim, homebrew_bin, cargo_bin, local_bin
    );

    let mut env = std::collections::HashMap::new();
    env.insert("PATH".to_string(), clean_path);
    env.insert("SSL_CERT_FILE".to_string(), ssl_cert.clone());
    env.insert("CURL_CA_BUNDLE".to_string(), ssl_cert);
    // Linux's x86 build-time relocation helper includes libelf definitions.
    // macOS has no system elf.h; Homebrew's libelf supplies the ABI types used
    // by MAN's conditional Linux host-tools patch.
    env.insert("CPATH".to_string(), libelf_include);
    env.insert("HOME".to_string(), home);
    // A script cannot be used as a shebang interpreter on macOS. Point
    // Autoconf/Automake at the real system Perl executable so generated host
    // tools remain directly executable instead of falling back to /bin/sh.
    env.insert("PERL".to_string(), "/usr/bin/perl".to_string());
    env.insert("PERL5OPT".to_string(), format!("-I{} -Mosx_fix", gnu_shim));
    // Autoconf uses TMPDIR to choose D-Bus' target session-socket directory.
    // Never let macOS' per-user /var/folders path leak into target binaries.
    env.insert("TMPDIR".to_string(), "/tmp".to_string());
    env
}

/// Environment variable names that must be **unset** (not just emptied) so
/// host Perl never inherits unrelated local modules.
pub const UNSET_ENV_VARS: &[&str] = &["PERL5LIB"];

/// Run a shell command and capture its output as a string.
#[allow(dead_code)]
pub fn capture_command(cmd: &str, args: &[&str], cwd: Option<&Path>) -> anyhow::Result<String> {
    let mut command = Command::new(cmd);
    command.args(args);
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }
    let output = command.output()?;
    if !output.status.success() {
        anyhow::bail!(
            "command {:?} failed with exit code {:?}: {}",
            cmd,
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
