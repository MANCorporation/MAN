//! The `.upd` file: an uncompressed tar holding, in this order,
//!   manifest.json       UpdateManifest
//!   manifest.sig        ed25519 signature of manifest.json, hex
//!   LICENSE.txt         license text shown before installing
//!   RELEASE-NOTES.txt   what is new
//!   system.tar.zst      the new system (@system contents)
//! The payload is covered by the signature through `payload_sha256`.
use anyhow::{anyhow, bail, Context, Result};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use man_common::*;
use std::process::{Command, Stdio};

pub const PAYLOAD: &str = "system.tar.zst";

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn unhex(s: &str) -> Result<Vec<u8>> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        bail!("bad hex length");
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| anyhow!("bad hex: {e}"))).collect()
}

/// Read one small member of the tar (the metadata files come first).
pub fn read_member(path: &str, name: &str) -> Result<Option<Vec<u8>>> {
    let out = Command::new("tar")
        .args(["-xOf", path, "--occurrence=1", name])
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .output()
        .context("cannot run tar")?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        if err.contains("Not found in archive") {
            return Ok(None);
        }
        bail!("This is not a valid MAN update file ({})", err.lines().next().unwrap_or("tar failed"));
    }
    Ok(Some(out.stdout))
}

/// (name, key) of every trusted update key.
pub fn trusted_keys() -> Vec<(String, VerifyingKey)> {
    let mut keys = vec![];
    if let Ok(dir) = std::fs::read_dir(UPDATE_KEYS_DIR) {
        for e in dir.filter_map(|e| e.ok()) {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("pub") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&p) else { continue };
            let Ok(bytes) = unhex(&text) else { continue };
            let Ok(arr) = <[u8; 32]>::try_from(bytes.as_slice()) else { continue };
            if let Ok(k) = VerifyingKey::from_bytes(&arr) {
                keys.push((p.file_stem().unwrap().to_string_lossy().into_owned(), k));
            }
        }
    }
    keys
}

pub struct Verified {
    pub manifest: UpdateManifest,
    pub signer: String,
    pub license: String,
    pub notes: String,
}

/// Check the file's signature against the trusted keys and read its texts.
pub fn verify(path: &str) -> Result<Verified> {
    let meta = std::fs::metadata(path).with_context(|| format!("cannot open {path}"))?;
    if !meta.is_file() {
        bail!("{path} is not a file.");
    }
    let manifest_bytes = read_member(path, "manifest.json")?.ok_or_else(|| anyhow!("This is not a MAN update file (no manifest)."))?;
    let sig_text = read_member(path, "manifest.sig")?.ok_or_else(|| anyhow!("The update is not signed, so it cannot be trusted."))?;
    let sig_bytes = unhex(&String::from_utf8_lossy(&sig_text))?;
    let sig = Signature::from_slice(&sig_bytes).map_err(|_| anyhow!("The update signature is damaged."))?;
    let signer = trusted_keys()
        .into_iter()
        .find(|(_, k)| k.verify(&manifest_bytes, &sig).is_ok())
        .map(|(n, _)| n)
        .ok_or_else(|| anyhow!("The update is not signed by MAN. It may have been modified or come from an untrusted source."))?;
    let manifest: UpdateManifest = serde_json::from_slice(&manifest_bytes).context("the update manifest is invalid")?;
    if manifest.format != UPD_FORMAT {
        bail!("This update needs a newer version of MAN Update (format {}).", manifest.format);
    }
    if !valid_version(&manifest.version) || manifest.payload_sha256.len() != 64 {
        bail!("The update manifest is invalid.");
    }
    let text = |name: &str| -> Result<String> { Ok(read_member(path, name)?.map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default()) };
    Ok(Verified { license: text("LICENSE.txt")?, notes: text("RELEASE-NOTES.txt")?, manifest, signer })
}
