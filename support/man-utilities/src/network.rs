use cosmic::app::{Core, Settings, Task};
use cosmic::iced::{Alignment, Length, Limits, Subscription};
use cosmic::{Application, Element, executor, theme, widget};
use std::fs;
use std::process::Command;
use std::time::Duration;

const REQUEST: &str = "/run/man-network.request";

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Settings::default()
        .size(cosmic::iced::Size::new(760.0, 520.0))
        .size_limits(Limits::NONE.min_width(620.0).min_height(420.0));
    cosmic::app::run::<NetworkApp>(settings, ())?;
    Ok(())
}

#[derive(Clone, Debug)]
enum Message {
    Refresh,
    Connect(String),
    Disconnect(String),
}

#[derive(Clone, Debug)]
struct Adapter {
    name: String,
    mac: String,
    address: Option<String>,
    up: bool,
}

struct NetworkApp {
    core: Core,
    adapters: Vec<Adapter>,
    status: String,
}

impl Application for NetworkApp {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "com.man.Network";

    fn core(&self) -> &Core { &self.core }
    fn core_mut(&mut self) -> &mut Core { &mut self.core }

    fn init(core: Core, _flags: ()) -> (Self, Task<Message>) {
        (
            Self { core, adapters: scan(), status: String::new() },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Refresh => self.adapters = scan(),
            Message::Connect(interface) => {
                self.status = request("connect", &interface);
                self.adapters = scan();
            }
            Message::Disconnect(interface) => {
                self.status = request("disconnect", &interface);
                self.adapters = scan();
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let mut list = widget::column::with_capacity(self.adapters.len() + 1).spacing(14);
        if self.adapters.is_empty() {
            list = list.push(widget::text::body("No wired network adapter detected."));
        }
        for adapter in &self.adapters {
            let connected = adapter.address.is_some();
            let subtitle = match &adapter.address {
                Some(address) => format!("Connected • {address}"),
                None if adapter.up => "Adapter detected • waiting for DHCP".into(),
                None => "Adapter is disconnected".into(),
            };
            let action = if connected {
                widget::button::standard("Disconnect")
                    .on_press(Message::Disconnect(adapter.name.clone()))
            } else {
                widget::button::suggested("Connect")
                    .on_press(Message::Connect(adapter.name.clone()))
            };
            let card = widget::container(
                widget::row::with_capacity(2)
                    .align_y(Alignment::Center)
                    .push(
                        widget::column::with_capacity(3)
                            .spacing(4)
                            .width(Length::Fill)
                            .push(widget::text::heading(&adapter.name))
                            .push(widget::text::body(subtitle))
                            .push(widget::text::caption(format!("MAC: {}", adapter.mac))),
                    )
                    .push(action),
            )
            .class(theme::Container::Secondary)
            .padding(18)
            .width(Length::Fill);
            list = list.push(card);
        }

        let content = widget::column::with_capacity(5)
            .spacing(18)
            .padding(28)
            .push(widget::text::title1("Network"))
            .push(widget::text::body("MAN Network manages wired and UTM virtio adapters through NetworkManager."))
            .push(list.width(Length::Fill))
            .push(widget::row::with_capacity(2).spacing(12)
                .push(widget::button::standard("Refresh").on_press(Message::Refresh))
                .push(widget::text::caption(&self.status)));
        widget::container(content)
            .class(theme::Container::WindowBackground)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        cosmic::iced::time::every(Duration::from_secs(3)).map(|_| Message::Refresh)
    }
}

fn scan() -> Vec<Adapter> {
    let Ok(entries) = fs::read_dir("/sys/class/net") else { return Vec::new() };
    let mut adapters = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "lo" { continue; }
        let base = entry.path();
        let mac = fs::read_to_string(base.join("address")).unwrap_or_default().trim().to_owned();
        let up = fs::read_to_string(base.join("operstate")).map(|v| v.trim() == "up").unwrap_or(false);
        let address = Command::new("/sbin/ip").args(["-4", "-o", "addr", "show", "dev", &name]).output().ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .and_then(|line| line.split_whitespace().skip_while(|word| *word != "inet").nth(1).map(str::to_owned));
        adapters.push(Adapter { name, mac, address, up });
    }
    adapters.sort_by(|a, b| a.name.cmp(&b.name));
    adapters
}

fn request(action: &str, interface: &str) -> String {
    if interface.is_empty() || !interface.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return "Invalid interface name.".into();
    }
    match fs::write(REQUEST, format!("{action} {interface}\n")) {
        Ok(()) => format!("{action} requested for {interface}"),
        Err(_) => "Network service is not ready.".into(),
    }
}
