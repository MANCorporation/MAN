//! Runs the privileged backend through pkexec and streams its events.
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use crate::i18n::tr;
use man_common::{Event, Request, BACKEND_PATH};
use std::ffi::OsStr;

pub struct Failure {
    pub message: String,
    pub detail: String,
}

impl Failure {
    fn new(message: impl Into<String>, detail: impl Into<String>) -> Self {
        Self { message: message.into(), detail: detail.into() }
    }
}

fn argv(program: String, args: &[&str], privileged: bool) -> Vec<String> {
    // MAN_NO_PKEXEC is for running the GUI itself as root in test VMs.
    let mut v = if !privileged || std::env::var_os("MAN_NO_PKEXEC").is_some() { vec![program] } else { vec!["pkexec".into(), program] };
    v.extend(args.iter().map(|a| a.to_string()));
    v
}

/// Send one request; `on_event` sees every event as it arrives. Resolves to
/// the backend's final message, or the error it reported.
pub async fn call(req: &Request, on_event: impl FnMut(&Event)) -> Result<String, Failure> {
    call_as(req, true, on_event).await
}

/// Run a read-only request as the current user, without asking for a password.
pub async fn call_unprivileged(req: &Request, on_event: impl FnMut(&Event)) -> Result<String, Failure> {
    call_as(req, false, on_event).await
}

async fn call_as(req: &Request, privileged: bool, on_event: impl FnMut(&Event)) -> Result<String, Failure> {
    // MAN_BACKEND lets developers run an uninstalled backend.
    let backend = std::env::var("MAN_BACKEND").unwrap_or_else(|_| BACKEND_PATH.to_string());
    call_program(argv(backend, &["request"], privileged), serde_json::to_vec(req).expect("request serializes"), on_event).await
}

/// Run another root helper of MAN through pkexec: `input` (one JSON request)
/// on stdin, events on stdout, as for the system helper.
pub async fn call_helper(program: &str, input: Vec<u8>, on_event: impl FnMut(&Event)) -> Result<String, Failure> {
    call_program(argv(program.to_string(), &[], true), input, on_event).await
}

async fn call_program(args: Vec<String>, input: Vec<u8>, mut on_event: impl FnMut(&Event)) -> Result<String, Failure> {
    let os: Vec<&OsStr> = args.iter().map(OsStr::new).collect();
    let flags = gio::SubprocessFlags::STDIN_PIPE | gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_SILENCE;
    let proc = gio::Subprocess::newv(&os, flags).map_err(|e| Failure::new(tr("Could not start the system helper."), e.to_string()))?;

    let stdin = proc.stdin_pipe().expect("stdin pipe");
    if let Err((_, e)) = stdin.write_all_future(input, glib::Priority::DEFAULT).await {
        return Err(Failure::new(tr("Could not talk to the system helper."), e.to_string()));
    }
    let _ = stdin.close_future(glib::Priority::DEFAULT).await;

    let stdout = gio::DataInputStream::new(&proc.stdout_pipe().expect("stdout pipe"));
    let mut outcome: Option<Result<String, Failure>> = None;
    loop {
        match stdout.read_line_utf8_future(glib::Priority::DEFAULT).await {
            Ok(Some(line)) => {
                let Ok(ev) = serde_json::from_str::<Event>(&line) else { continue };
                match &ev {
                    Event::Done { message } => outcome = Some(Ok(message.clone())),
                    // Fixed messages of the helper are in the catalog too.
                    Event::Error { message, detail } => outcome = Some(Err(Failure::new(crate::i18n::tr_dyn(message), detail))),
                    _ => {}
                }
                on_event(&ev);
            }
            Ok(None) => break,
            Err(e) => {
                outcome.get_or_insert(Err(Failure::new(tr("Lost contact with the system helper."), e.to_string())));
                break;
            }
        }
    }
    let _ = proc.wait_future().await;
    outcome.unwrap_or_else(|| {
        // pkexec: 126 = authorization dismissed, 127 = not authorized.
        let code = if proc.has_exited() { proc.exit_status() } else { -1 };
        if code == 126 || code == 127 {
            Err(Failure::new(tr("Authorization was cancelled."), tr("Administrator rights are needed for this action.")))
        } else {
            Err(Failure::new(tr("The system helper stopped unexpectedly."), format!("Exit status {code}")))
        }
    })
}
