use cosmic::app::{Core, Settings, Task};
use cosmic::iced::{Alignment, Length, Limits};
use cosmic::{executor, theme, widget, Application, Element};
use std::fs;

const FALLBACK_GUIDE: &str = "/usr/share/man/help/MAN-HELP.txt";

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Settings::default()
        .size(cosmic::iced::Size::new(900.0, 680.0))
        .size_limits(Limits::NONE.min_width(640.0).min_height(440.0));
    cosmic::app::run::<GuideApp>(settings, ())?;
    Ok(())
}

#[derive(Clone, Debug)]
enum Message {}

struct GuideApp {
    core: Core,
    text: String,
}

impl Application for GuideApp {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "com.man.Guide";

    fn core(&self) -> &Core { &self.core }
    fn core_mut(&mut self) -> &mut Core { &mut self.core }

    fn init(mut core: Core, _flags: ()) -> (Self, Task<Message>) {
        core.window.show_headerbar = true;
        core.window.show_close = true;
        core.window.show_maximize = true;
        core.window.show_minimize = true;
        core.set_header_title("MAN Guide".into());
        let locale = std::env::var("LANG")
            .ok()
            .and_then(|value| value.split('.').next().map(str::to_owned))
            .unwrap_or_else(|| "en_US".to_owned());
        let localized = format!("/usr/share/man/help/{locale}/MAN-HELP.txt");
        let text = fs::read_to_string(&localized)
            .or_else(|_| fs::read_to_string(FALLBACK_GUIDE))
            .unwrap_or_else(|_| "MAN Guide is unavailable.\n\nMAN-GUIDE-0001".to_owned());
        (Self { core, text }, Task::none())
    }

    fn update(&mut self, _: Message) -> Task<Message> { Task::none() }

    fn view(&self) -> Element<'_, Message> {
        let content = widget::column::with_capacity(3)
            .spacing(16)
            .padding(28)
            .push(widget::text::title1("MAN Guide"))
            .push(widget::text::body("Offline help, installation guidance, and error-code explanations."))
            .push(widget::scrollable(
                widget::container(widget::text::body(&self.text))
                    .padding(20)
                    .width(Length::Fill)
                    .class(theme::Container::Secondary),
            ).height(Length::Fill));
        widget::container(content)
            .class(theme::Container::WindowBackground)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Start)
            .into()
    }
}
