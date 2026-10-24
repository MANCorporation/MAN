//! Privileged backend of the MAN installer and snapshot manager.
//!
//!   man-installer-backend request          JSON Request on stdin, JSON-line Events on stdout (via pkexec)
//!   man-installer-backend snapshot-auto K  K = daily | pre-update (systemd timer, apt hook)
//!   man-installer-backend boot-cleanup     after a rollback (systemd, at boot)
mod install;
mod locale;
mod oobe;
mod probe;
mod snapshots;
mod update;
mod updfile;
mod util;

use man_common::{Event, Request};
use std::io::Read;
use util::{emit, info, log, open_log};

fn handle(req: Request) -> anyhow::Result<String> {
    match req {
        Request::Probe => {
            emit(&Event::Probe { probe: probe::probe()? });
            Ok(String::new())
        }
        Request::Inspect { device, passphrase } => {
            emit(&Event::Inspect { install: probe::inspect_encrypted(&device, &passphrase)? });
            Ok(String::new())
        }
        Request::Install { plan } => install::install(plan),
        Request::SnapshotCreate { description } => {
            let s = snapshots::create(&description, "manual", true)?;
            Ok(format!("Snapshot “{}” created.", s.description))
        }
        Request::SnapshotDelete { id } => {
            snapshots::delete(&id)?;
            Ok("Snapshot deleted.".into())
        }
        Request::Rollback { id } => snapshots::rollback(&id),
        Request::SetSnapshotConfig { config } => {
            snapshots::set_config(&config)?;
            Ok("Snapshot settings saved.".into())
        }
        Request::OobeApply { config } => oobe::apply(config),
        Request::UpdateInspect { path } => {
            emit(&Event::Update { info: update::inspect(&path)? });
            Ok(String::new())
        }
        Request::UpdateStage { path, device, snapshot } => update::stage(&path, &device, snapshot),
        Request::SetLiveLanguage { locale } => locale::set_live_language(&locale),
    }
}

fn request() -> i32 {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        emit(&Event::Error { message: "Could not read the request.".into(), detail: String::new() });
        return 2;
    }
    let req: Request = match serde_json::from_str(&input) {
        Ok(r) => r,
        Err(e) => {
            emit(&Event::Error { message: "The request was not understood.".into(), detail: e.to_string() });
            return 2;
        }
    };
    let op = match &req {
        Request::Install { .. } => "install",
        Request::UpdateStage { .. } => "update",
        Request::Probe => "probe",
        _ => "request",
    };
    log(&format!("--- {op} started"));
    match handle(req) {
        Ok(msg) => {
            log(&format!("--- {op} finished: {msg}"));
            emit(&Event::Done { message: msg });
            0
        }
        Err(e) => {
            log(&format!("--- {op} failed: {e:#}"));
            emit(&Event::Error { message: format!("{e}"), detail: format!("{e:#}\n\nFull log: {}", util::LOG_FILE) });
            1
        }
    }
}

fn report(r: anyhow::Result<()>) -> i32 {
    match r {
        Ok(()) => 0,
        Err(e) => {
            info(format!("failed: {e:#}"));
            eprintln!("man: {e:#}");
            1
        }
    }
}

fn main() {
    open_log();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["request"] => request(),
        ["snapshot-auto", kind] => match snapshots::auto(kind) {
            Ok(()) => 0,
            Err(e) => {
                info(format!("automatic snapshot failed: {e:#}"));
                eprintln!("man snapshot: {e:#}");
                0 // never block apt or the timer on a snapshot problem
            }
        },
        ["oobe-cleanup"] => report(oobe::cleanup()),
        ["update-finish"] => report(update::finish()),
        ["encrypt-online"] => report(oobe::encrypt_online()),
        ["boot-cleanup"] => match snapshots::boot_cleanup() {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("man boot cleanup: {e:#}");
                1
            }
        },
        _ => {
            eprintln!("usage: man-installer-backend request | snapshot-auto daily|pre-update | boot-cleanup | oobe-cleanup | encrypt-online | update-finish");
            2
        }
    };
    std::process::exit(code);
}
