//! man-mkupd: create signed MAN update files.
//!
//!   man-mkupd keygen PRIVATE PUBLIC
//!       New signing key pair (hex). Keep PRIVATE secret; install PUBLIC as
//!       /usr/share/man/update-keys/<name>.pub on MAN systems.
//!   man-mkupd payload ROOTDIR OUTDIR          (as root)
//!       Pack an installed-system tree into OUTDIR/system.tar.zst and
//!       OUTDIR/payload.json (checksum and sizes).
//!   man-mkupd pack --version V --payload OUTDIR --key PRIVATE [--license F] [--notes F] [--name N] -o FILE.upd
//!       Write and sign the manifest, assemble the .upd.
#[path = "updfile.rs"]
mod updfile;

use anyhow::{anyhow, bail, Context, Result};
use ed25519_dalek::{Signer, SigningKey};
use man_common::*;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};

/// Never part of a system image (same idea as the installer's exclusions).
const EXCLUDES: &[&str] = &[
    "./dev/*", "./proc/*", "./sys/*", "./run/*", "./tmp/*", "./var/tmp/*", "./mnt/*", "./media/*", "./lost+found",
    "./home/*", "./root/*", "./.snapshots/*", "./swapfile", "./boot/efi/*", "./var/cache/apt/archives/*.deb",
    "./var/log/journal/*", "./var/lib/systemd/random-seed", "./var/lib/AccountsService/users/MANInstaller",
    "./var/lib/gdm3/.cache/*", "./var/log/man-installer.log", "./boot/grub/*", "./boot/man-update",
    "./etc/machine-id", "./var/lib/dbus/machine-id",
];

fn keygen(private: &str, public: &str) -> Result<()> {
    let mut seed = [0u8; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut seed)?;
    let key = SigningKey::from_bytes(&seed);
    if Path::new(private).exists() {
        bail!("{private} already exists; refusing to overwrite a signing key");
    }
    std::fs::write(private, updfile::hex(&seed) + "\n")?;
    std::fs::set_permissions(private, std::os::unix::fs::PermissionsExt::from_mode(0o600))?;
    std::fs::write(public, updfile::hex(key.verifying_key().as_bytes()) + "\n")?;
    println!("private key: {private} (keep it secret, back it up)\npublic key:  {public}");
    Ok(())
}

fn du(root: &str) -> u64 {
    Command::new("du")
        .args(["-sxb", root])
        .output()
        .ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).split_whitespace().next().and_then(|n| n.parse().ok()))
        .unwrap_or(0)
}

fn payload(root: &str, outdir: &str) -> Result<()> {
    std::fs::create_dir_all(outdir)?;
    let out = Path::new(outdir).join(updfile::PAYLOAD);
    let mut args: Vec<String> = vec!["--zstd".into(), "-cpf".into(), "-".into(), "--xattrs".into(), "--xattrs-include=*".into(), "--acls".into(), "--numeric-owner".into(), "--one-file-system".into()];
    args.extend(EXCLUDES.iter().map(|e| format!("--exclude={e}")));
    args.extend(["-C".into(), root.into(), ".".into()]);
    eprintln!("packing {root} ...");
    let mut child = Command::new("tar").args(&args).env("ZSTD_CLEVEL", "6").env("ZSTD_NBTHREADS", "0").stdout(Stdio::piped()).spawn().context("cannot run tar")?;
    let mut src = child.stdout.take().unwrap();
    let mut file = std::fs::File::create(&out)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    let mut size = 0u64;
    loop {
        let n = src.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        std::io::Write::write_all(&mut file, &buf[..n])?;
        size += n as u64;
    }
    let st = child.wait()?;
    // tar exit 1 = some files changed while reading (harmless for an offline tree).
    if !(st.success() || st.code() == Some(1)) {
        bail!("tar failed: {st}");
    }
    let info = serde_json::json!({ "payload_sha256": updfile::hex(&hasher.finalize()), "payload_size": size, "installed_size": du(root) });
    std::fs::write(Path::new(outdir).join("payload.json"), serde_json::to_string_pretty(&info)?)?;
    println!("{}: {} bytes\n{}", out.display(), size, info);
    Ok(())
}

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

fn pack(args: &[String]) -> Result<()> {
    let version = arg(args, "--version").ok_or_else(|| anyhow!("--version is required"))?;
    if !valid_version(&version) {
        bail!("invalid version {version}");
    }
    let pdir = arg(args, "--payload").ok_or_else(|| anyhow!("--payload is required"))?;
    let key_path = arg(args, "--key").ok_or_else(|| anyhow!("--key is required"))?;
    let out = arg(args, "-o").ok_or_else(|| anyhow!("-o is required"))?;
    let p: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(Path::new(&pdir).join("payload.json"))?)?;
    let seed = updfile::unhex(&std::fs::read_to_string(&key_path)?)?;
    let key = SigningKey::from_bytes(&<[u8; 32]>::try_from(seed.as_slice()).map_err(|_| anyhow!("bad key"))?);
    let today = Command::new("date").arg("+%F").output().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
    let manifest = UpdateManifest {
        format: UPD_FORMAT,
        name: arg(args, "--name").unwrap_or_else(|| "MAN".into()),
        version,
        date: today,
        payload_sha256: p["payload_sha256"].as_str().unwrap_or_default().into(),
        payload_size: p["payload_size"].as_u64().unwrap_or(0),
        installed_size: p["installed_size"].as_u64().unwrap_or(0),
    };
    let stage = Path::new(&out).with_extension("upd.parts");
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage)?;
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
    std::fs::write(stage.join("manifest.json"), &manifest_bytes)?;
    std::fs::write(stage.join("manifest.sig"), updfile::hex(&key.sign(&manifest_bytes).to_bytes()) + "\n")?;
    std::fs::write(stage.join("LICENSE.txt"), arg(args, "--license").map(std::fs::read_to_string).transpose()?.unwrap_or_default())?;
    std::fs::write(stage.join("RELEASE-NOTES.txt"), arg(args, "--notes").map(std::fs::read_to_string).transpose()?.unwrap_or_default())?;
    let payload_src = std::fs::canonicalize(Path::new(&pdir).join(updfile::PAYLOAD))?;
    std::os::unix::fs::symlink(&payload_src, stage.join(updfile::PAYLOAD))?;
    let st = Command::new("tar")
        .args(["--format=posix", "--owner=0", "--group=0", "--mode=0644", "--dereference", "-cf"])
        .arg(&out)
        .arg("-C")
        .arg(&stage)
        .args(["manifest.json", "manifest.sig", "LICENSE.txt", "RELEASE-NOTES.txt", updfile::PAYLOAD])
        .status()?;
    std::fs::remove_dir_all(&stage)?;
    if !st.success() {
        bail!("tar failed");
    }
    println!("{out}: MAN {} signed", manifest.version);
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let r = match args.first().map(String::as_str) {
        Some("keygen") if args.len() == 3 => keygen(&args[1], &args[2]),
        Some("payload") if args.len() == 3 => payload(&args[1], &args[2]),
        Some("pack") => pack(&args[1..]),
        Some("verify") if args.len() == 2 => updfile::verify(&args[1]).map(|v| println!("OK: {} {} signed by {} (license {} bytes, notes {} bytes)", v.manifest.name, v.manifest.version, v.signer, v.license.len(), v.notes.len())),
        _ => {
            eprintln!("usage: man-mkupd keygen PRIV PUB | payload ROOT OUTDIR | pack --version V --payload DIR --key PRIV [--license F] [--notes F] -o FILE.upd | verify FILE.upd");
            std::process::exit(2);
        }
    };
    if let Err(e) = r {
        eprintln!("man-mkupd: {e:#}");
        std::process::exit(1);
    }
}
