//! Network helpers shared by MAN Setup and MAN Recovery: connection state and
//! Wi-Fi networks through nmcli.
use crate::i18n::{tr, trf};
use adw::prelude::*;
use gtk::{gio, glib};
use std::cell::RefCell;
use std::rc::Rc;

pub fn command(cmd: &str, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new(cmd).args(args).env("LC_ALL", "C.UTF-8").output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[derive(Clone, Debug)]
pub struct WifiNet {
    pub ssid: String,
    pub signal: u32,
    pub secure: bool,
    pub in_use: bool,
}

/// Split one `nmcli -t` line, honoring `\:` escapes.
pub fn nm_fields(line: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(n) = chars.next() {
                    out.last_mut().unwrap().push(n);
                }
            }
            ':' => out.push(String::new()),
            c => out.last_mut().unwrap().push(c),
        }
    }
    out
}

pub struct NetStatus {
    pub connected: bool,
    pub description: String,
    pub wifi_device: bool,
}

pub fn net_status() -> NetStatus {
    let devices = command("nmcli", &["-t", "-f", "DEVICE,TYPE,STATE,CONNECTION", "device"]).unwrap_or_default();
    let mut connected = None;
    let mut wifi_device = false;
    for l in devices.lines() {
        let f = nm_fields(l);
        if f.len() < 4 {
            continue;
        }
        if f[1] == "wifi" {
            wifi_device = true;
        }
        if f[2] == "connected" && (f[1] == "ethernet" || f[1] == "wifi") && connected.is_none() {
            let kind = if f[1] == "wifi" { "Wi-Fi" } else { tr("Wired") };
            connected = Some(format!("{kind}: {}", f[3]));
        }
    }
    match connected {
        Some(d) => NetStatus { connected: true, description: d, wifi_device },
        None => NetStatus { connected: false, description: tr("Not connected").into(), wifi_device },
    }
}

pub fn wifi_networks() -> Vec<WifiNet> {
    let out = command("nmcli", &["-t", "-f", "IN-USE,SSID,SIGNAL,SECURITY", "device", "wifi", "list", "--rescan", "auto"]).unwrap_or_default();
    let mut nets: Vec<WifiNet> = vec![];
    for l in out.lines() {
        let f = nm_fields(l);
        if f.len() < 4 || f[1].is_empty() {
            continue;
        }
        let net = WifiNet { in_use: f[0] == "*", ssid: f[1].clone(), signal: f[2].parse().unwrap_or(0), secure: !f[3].is_empty() && f[3] != "--" };
        match nets.iter_mut().find(|n| n.ssid == net.ssid) {
            Some(n) if n.signal >= net.signal => {}
            Some(n) => *n = net,
            None => nets.push(net),
        }
    }
    nets.sort_by(|a, b| b.in_use.cmp(&a.in_use).then(b.signal.cmp(&a.signal)));
    nets
}

pub fn signal_icon(s: u32) -> &'static str {
    match s {
        0..=24 => "network-wireless-signal-weak-symbolic",
        25..=49 => "network-wireless-signal-ok-symbolic",
        50..=74 => "network-wireless-signal-good-symbolic",
        _ => "network-wireless-signal-excellent-symbolic",
    }
}

/// Connect to `net`, asking for its password when it has one.
pub fn connect_wifi(window: &impl IsA<gtk::Widget>, toast: Rc<dyn Fn(&str)>, row: &adw::ActionRow, net: &WifiNet, update: Rc<dyn Fn()>, refill: Rc<RefCell<Box<dyn Fn()>>>) {
    let ssid = net.ssid.clone();
    let do_connect = {
        let toast = toast.clone();
        let row = row.clone();
        move |password: Option<String>| {
            let (toast, row, update, refill, ssid) = (toast.clone(), row.clone(), update.clone(), refill.clone(), ssid.clone());
            row.set_sensitive(false);
            let spinner = adw::Spinner::new();
            row.add_suffix(&spinner);
            glib::spawn_future_local(async move {
                let s2 = ssid.clone();
                let ok = gio::spawn_blocking(move || {
                    let mut args = vec!["device", "wifi", "connect", s2.as_str()];
                    if let Some(p) = password.as_deref() {
                        args.extend(["password", p]);
                    }
                    command("nmcli", &args).is_some()
                })
                .await
                .unwrap_or(false);
                row.remove(&spinner);
                row.set_sensitive(true);
                toast(&if ok { trf("Connected to “{ssid}”", &[("ssid", &ssid)]) } else { trf("Could not connect to “{ssid}”", &[("ssid", &ssid)]) });
                update();
                (refill.borrow())();
            });
        }
    };
    if !net.secure {
        do_connect(None);
        return;
    }
    let dialog = adw::AlertDialog::new(Some(&trf("Connect to “{ssid}”", &[("ssid", &net.ssid)])), Some(tr("Enter the network password.")));
    let entry = gtk::PasswordEntry::builder().show_peek_icon(true).activates_default(true).build();
    dialog.set_extra_child(Some(&entry));
    dialog.add_response("cancel", tr("_Cancel"));
    dialog.add_response("connect", tr("C_onnect"));
    dialog.set_response_appearance("connect", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("connect"));
    dialog.set_close_response("cancel");
    let window = window.clone().upcast::<gtk::Widget>();
    glib::spawn_future_local(async move {
        let e = entry.clone();
        glib::idle_add_local_once(move || {
            e.grab_focus();
        });
        if dialog.choose_future(Some(&window)).await == "connect" {
            do_connect(Some(entry.text().to_string()));
        }
    });
}

