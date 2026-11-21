//! MAN's corner of the EFI partition (/boot/efi/EFI/MAN), which macOS can read
//! and write too: DUHACH! for Mac uses it to show MAN's snapshots and to ask MAN
//! to restore one. (GRUB keeps its start-up options in bootmenu.env there.)
//!   snapshots.json   the snapshot list, written whenever it changes
//!   restore-request  a snapshot id: restore it at the next start of MAN
//!   restore-result   what happened, for DUHACH! to show
use crate::snapshots;
use crate::util::*;
use anyhow::{bail, Result};
use std::path::Path;

pub const ESP_DIR: &str = "/boot/efi/EFI/MAN";

fn esp_mounted() -> bool {
    query("findmnt", &["-n", "-o", "FSTYPE", "--mountpoint", "/boot/efi"]).is_some_and(|t| t.trim() == "vfat")
}

/// Writes a file on the FAT partition: a temporary file renamed over the old one.
fn put(name: &str, text: &str) -> Result<()> {
    let dir = Path::new(ESP_DIR);
    std::fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(".{name}.tmp"));
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, dir.join(name))?;
    Ok(())
}

/// The snapshot list for DUHACH! for Mac. Never fails the caller.
pub fn export_snapshots() {
    if !esp_mounted() {
        return;
    }
    let version = read_to_string("/usr/lib/man/version").unwrap_or_default();
    let doc = serde_json::json!({
        "man_version": version.trim(),
        "snapshots": snapshots::list(),
    });
    if let Err(e) = put("snapshots.json", &(doc.to_string() + "\n")) {
        info(format!("Warning: could not write the snapshot list to the EFI partition: {e:#}"));
    }
}

/// At start-up: carry out a restore asked for in macOS. Returns true when MAN
/// must restart into the restored system.
pub fn handle_requests() -> Result<bool> {
    let request = Path::new(ESP_DIR).join("restore-request");
    let Some(text) = read_to_string(&request) else { return Ok(false) };
    // Removed first: a problem must never make every start try again.
    std::fs::remove_file(&request)?;
    let id = text.trim().to_string();
    let result = (|| -> Result<String> {
        if !snapshots::list().iter().any(|s| s.id == id) {
            bail!("The snapshot {id} does not exist any more.");
        }
        snapshots::rollback(&id)
    })();
    let now = rfc3339(now());
    match &result {
        Ok(msg) => {
            info(format!("Restore asked for in macOS: {msg}"));
            let _ = put("restore-result", &format!("ok\n{now}\n{id}\n"));
        }
        Err(e) => {
            info(format!("Restore asked for in macOS failed: {e:#}"));
            let _ = put("restore-result", &format!("failed\n{now}\n{id}\n{e:#}\n"));
        }
    }
    export_snapshots();
    Ok(result.is_ok())
}
