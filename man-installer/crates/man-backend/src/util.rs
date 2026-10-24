use anyhow::{anyhow, bail, Context, Result};
use man_common::Event;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub const LOG_FILE: &str = "/var/log/man-installer.log";

static LOG: Mutex<Option<File>> = Mutex::new(None);

pub fn open_log() {
    let file = OpenOptions::new().create(true).append(true).open(LOG_FILE).ok();
    *LOG.lock().unwrap() = file;
}

pub fn log(line: &str) {
    if let Some(f) = LOG.lock().unwrap().as_mut() {
        let _ = writeln!(f, "[{}] {}", rfc3339(now()), line);
    }
}

pub fn emit(ev: &Event) {
    if let Event::Log { line } = ev {
        log(line);
    }
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{}", serde_json::to_string(ev).unwrap());
    let _ = out.flush();
}

pub fn info(line: impl Into<String>) {
    emit(&Event::Log { line: line.into() });
}

fn describe(cmd: &str, args: &[&str]) -> String {
    std::iter::once(cmd).chain(args.iter().copied()).collect::<Vec<_>>().join(" ")
}

/// Run a command, log it, return stdout. Fails with stderr on non-zero exit.
pub fn run(cmd: &str, args: &[&str]) -> Result<String> {
    run_input(cmd, args, None)
}

pub fn run_input(cmd: &str, args: &[&str], input: Option<&[u8]>) -> Result<String> {
    let shown = describe(cmd, args);
    info(format!("$ {shown}"));
    let mut child = Command::new(cmd)
        .args(args)
        .env("LC_ALL", "C.UTF-8")
        .stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("cannot start {cmd}"))?;
    if let Some(data) = input {
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(data)?;
    }
    let out = child.wait_with_output()?;
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    for l in stderr.lines().filter(|l| !l.trim().is_empty()).take(40) {
        log(&format!("  {l}"));
    }
    if !out.status.success() {
        let tail: Vec<&str> = stderr.lines().rev().take(6).collect::<Vec<_>>().into_iter().rev().collect();
        bail!("{} failed ({}): {}", shown, out.status, tail.join(" / "));
    }
    Ok(stdout)
}

/// Quiet variant for probing: no logging, no error on failure.
pub fn query(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).env("LC_ALL", "C.UTF-8").stderr(Stdio::null()).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn chroot(root: &Path, args: &[&str]) -> Result<String> {
    let mut full = vec![root.to_str().unwrap()];
    full.extend_from_slice(args);
    run("chroot", &full)
}

pub fn chroot_input(root: &Path, args: &[&str], input: &[u8]) -> Result<String> {
    let mut full = vec![root.to_str().unwrap()];
    full.extend_from_slice(args);
    run_input("chroot", &full, Some(input))
}

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// (year, month, day, hour, minute, second) in UTC.
pub fn civil(ts: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = (ts / 86400) as i64;
    let rem = ts % 86400;
    // Howard Hinnant's days-from-civil inverse.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (y, m, d, (rem / 3600) as u32, (rem % 3600 / 60) as u32, (rem % 60) as u32)
}

pub fn rfc3339(ts: u64) -> String {
    let (y, mo, d, h, mi, s) = civil(ts);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// Snapshot ids are UTC timestamps: `YYYYMMDD-HHMMSS`.
pub fn snapshot_id(ts: u64) -> String {
    let (y, mo, d, h, mi, s) = civil(ts);
    format!("{y:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}")
}

pub fn read_to_string(path: impl AsRef<Path>) -> Option<String> {
    let mut s = String::new();
    File::open(path).ok()?.read_to_string(&mut s).ok()?;
    Some(s)
}

pub fn write_file(path: impl AsRef<Path>, content: &str, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let path = path.as_ref();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, content).with_context(|| format!("writing {}", path.display()))?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    Ok(())
}

/// Mount points created by an operation, unmounted in reverse order on drop
/// so a failure never leaves the target mounted.
#[derive(Default)]
pub struct Mounts {
    points: Vec<PathBuf>,
    mappers: Vec<String>,
}

impl Mounts {
    pub fn mount(&mut self, src: &str, dst: &Path, opts: &[&str]) -> Result<()> {
        std::fs::create_dir_all(dst)?;
        let mut args: Vec<&str> = opts.to_vec();
        args.push(src);
        args.push(dst.to_str().ok_or_else(|| anyhow!("bad path"))?);
        run("mount", &args)?;
        self.points.push(dst.to_path_buf());
        Ok(())
    }

    pub fn luks_open(&mut self, device: &str, name: &str, passphrase: &str, read_only: bool) -> Result<String> {
        let mut args = vec!["open", "--type", "luks", "--key-file=-", device, name];
        if read_only {
            args.insert(1, "--readonly");
        }
        run_input("cryptsetup", &args, Some(passphrase.as_bytes()))
            .map_err(|_| anyhow!("The passphrase is not correct, or the encrypted partition cannot be opened."))?;
        self.mappers.push(name.to_string());
        Ok(format!("/dev/mapper/{name}"))
    }

    pub fn release(&mut self) {
        let _ = Command::new("sync").status();
        while let Some(p) = self.points.pop() {
            let ok = Command::new("umount").arg(&p).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false);
            if !ok {
                log(&format!("lazy unmount of {}", p.display()));
                let _ = Command::new("umount").arg("-l").arg(&p).status();
            }
        }
        while let Some(m) = self.mappers.pop() {
            let _ = Command::new("cryptsetup").args(["close", &m]).stderr(Stdio::null()).status();
        }
    }
}

impl Drop for Mounts {
    fn drop(&mut self) {
        self.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(1_792_337_412), "2026-10-18T15:30:12Z");
        assert_eq!(snapshot_id(1_792_337_412), "20261018-153012");
        assert_eq!(rfc3339(951_782_400), "2000-02-29T00:00:00Z");
    }
}
