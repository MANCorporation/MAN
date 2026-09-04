use crate::i18n;
use cosmic::app::{Core, Settings, Task};
use cosmic::iced::window::{self, set_mode};
use cosmic::iced::{Alignment, Background, Color, Length, Limits, Subscription};
use cosmic::{Application, Element, executor, theme, widget};
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};
use std::time::Duration;

const STATUS: &str = "/run/man-oobe.status";
const REQUEST: &str = "/run/man-oobe.request";
const PASSWORD_FILE: &str = "/run/man-oobe.password";
const LICENSE: &str = "MAN SOFTWARE NOTICE\n\nMAN-authored components are proprietary: Copyright (c) 2026 MAN project contributors. All Rights Reserved. This notice does not apply to included open-source software, which retains its own copyright notices and license terms. Third-party notices and release compliance information are available in /usr/share/doc/MAN.\n\nTHE SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND. You are responsible for backups, account credentials, and the configuration of this computer.\n\nBy selecting ‘I agree’, you accept the applicable third-party terms.";

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Settings::default()
        .size_limits(Limits::NONE)
        .exit_on_close(false)
        .client_decorations(false);
    cosmic::app::run::<Oobe>(settings, ())?;
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    Welcome,
    License,
    Account,
    Language,
    Region,
    Keyboard,
    Theme,
    Accent,
    Wallpaper,
    Layout,
    Dock,
    Accessibility,
    Privacy,
    Updates,
    Timezone,
    Gestures,
    Apps,
    Review,
    Applying,
    Done,
    Error,
}

impl Page {
    const FULL: [Self; 18] = [
        Self::Welcome,
        Self::License,
        Self::Account,
        Self::Language,
        Self::Region,
        Self::Keyboard,
        Self::Theme,
        Self::Accent,
        Self::Wallpaper,
        Self::Layout,
        Self::Dock,
        Self::Accessibility,
        Self::Privacy,
        Self::Updates,
        Self::Timezone,
        Self::Gestures,
        Self::Apps,
        Self::Review,
    ];

    fn title(self) -> &'static str {
        match self {
            Self::Welcome => "Welcome to MAN",
            Self::License => "License Agreement",
            Self::Account => "Create Your Account",
            Self::Language => "Language",
            Self::Region => "Region & Formats",
            Self::Keyboard => "Keyboard",
            Self::Theme => "Choose a Theme",
            Self::Accent => "Accent Color",
            Self::Wallpaper => "Desktop Background",
            Self::Layout => "Desktop Layout",
            Self::Dock => "Dock Behavior",
            Self::Accessibility => "Accessibility",
            Self::Privacy => "Privacy",
            Self::Updates => "Updates",
            Self::Timezone => "Date & Time",
            Self::Gestures => "Gestures & Navigation",
            Self::Apps => "Favorite Applications",
            Self::Review => "Ready for MAN",
            Self::Applying => "Setting Up MAN",
            Self::Done => "MAN Is Ready",
            Self::Error => "Setup Could Not Finish",
        }
    }

    fn title_tr(self) -> String {
        match self {
            Self::Welcome => i18n::tr("oobe_welcome"),
            Self::License => i18n::tr("oobe_license_short"),
            Self::Account => i18n::tr("oobe_account"),
            Self::Language => i18n::tr("oobe_language"),
            Self::Region => i18n::tr("oobe_region_short"),
            Self::Keyboard => i18n::tr("oobe_keyboard"),
            Self::Theme => i18n::tr("oobe_theme"),
            Self::Accent => i18n::tr("oobe_accent"),
            Self::Wallpaper => i18n::tr("oobe_wallpaper"),
            Self::Layout => i18n::tr("oobe_layout_short"),
            Self::Dock => i18n::tr("oobe_dock"),
            Self::Accessibility => i18n::tr("oobe_accessibility"),
            Self::Privacy => i18n::tr("oobe_privacy"),
            Self::Updates => i18n::tr("oobe_updates"),
            Self::Timezone => i18n::tr("oobe_timezone_short"),
            Self::Gestures => i18n::tr("oobe_gestures"),
            Self::Apps => i18n::tr("oobe_apps"),
            Self::Review => i18n::tr("oobe_review"),
            Self::Applying => i18n::tr("oobe_applying"),
            Self::Done => i18n::tr("oobe_done"),
            Self::Error => i18n::tr("oobe_error"),
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Welcome => "com.man.Utilities",
            Self::License => "text-x-generic-symbolic",
            Self::Account => "system-users",
            Self::Language => "preferences-desktop-locale",
            Self::Region => "mark-location-symbolic",
            Self::Keyboard => "preferences-desktop-keyboard",
            Self::Theme => "preferences-desktop-appearance",
            Self::Accent => "applications-graphics",
            Self::Wallpaper => "preferences-desktop-wallpaper",
            Self::Layout => "view-grid-symbolic",
            Self::Dock => "preferences-desktop",
            Self::Accessibility => "preferences-desktop-accessibility",
            Self::Privacy => "preferences-system-privacy",
            Self::Updates => "preferences-system-updates",
            Self::Timezone => "preferences-system-time",
            Self::Gestures => "preferences-touchpad",
            Self::Apps => "applications-other",
            Self::Review => "emblem-ok-symbolic",
            Self::Applying => "system-run-symbolic",
            Self::Done => "emblem-ok-symbolic",
            Self::Error => "dialog-error-symbolic",
        }
    }
}

#[derive(Clone, Debug)]
enum Message {
    Start,
    Skip,
    Back,
    Next,
    Agree(bool),
    FullName(String),
    Username(String),
    Password(String),
    Confirm(String),
    TogglePassword,
    ToggleConfirm,
    Choose(Choice, String),
    Toggle(Toggle),
    Apply,
    Poll,
    EnsureFullscreen,
    EnterDesktop,
    Retry,
    ThemeFadeTick,
}

#[derive(Clone, Copy, Debug)]
enum Choice {
    Language,
    Region,
    Keyboard,
    Theme,
    Accent,
    Wallpaper,
    Layout,
    Dock,
    Timezone,
}

#[derive(Clone, Copy, Debug)]
enum Toggle {
    LargeText,
    HighContrast,
    ReducedMotion,
    ScreenReader,
    Location,
    Diagnostics,
    AutoUpdates,
    Gestures,
    Tiling,
    Terminal,
    Files,
    Settings,
}

struct Oobe {
    core: Core,
    page: Page,
    quick: bool,
    agreed: bool,
    full_name: String,
    username: String,
    password: String,
    confirm: String,
    password_hidden: bool,
    confirm_hidden: bool,
    language: String,
    region: String,
    keyboard: String,
    theme: String,
    accent: String,
    wallpaper: String,
    layout: String,
    dock: String,
    timezone: String,
    large_text: bool,
    high_contrast: bool,
    reduced_motion: bool,
    screen_reader: bool,
    location: bool,
    diagnostics: bool,
    auto_updates: bool,
    gestures: bool,
    tiling: bool,
    favorite_terminal: bool,
    favorite_files: bool,
    favorite_settings: bool,
    status: String,
    pending_theme: Option<String>,
    theme_switched: bool,
    theme_fade: f32,
}

impl Application for Oobe {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "com.man.OOBE";

    fn core(&self) -> &Core {
        &self.core
    }
    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(mut core: Core, _flags: ()) -> (Self, Task<Message>) {
        core.window.show_headerbar = false;
        core.window.show_close = false;
        core.window.show_maximize = false;
        core.window.show_minimize = false;
        let immediate = core
            .main_window_id()
            .map(|id| set_mode(id, window::Mode::Fullscreen))
            .unwrap_or_else(Task::none);
        let delayed = Task::perform(tokio::time::sleep(Duration::from_millis(700)), |_| {
            cosmic::Action::App(Message::EnsureFullscreen)
        });
        (
            Self {
                core,
                page: Page::Welcome,
                quick: false,
                agreed: false,
                full_name: String::new(),
                username: String::new(),
                password: String::new(),
                confirm: String::new(),
                password_hidden: true,
                confirm_hidden: true,
                // The installer already saved a stable locale ID. Show that
                // language in OOBE instead of resetting the form to English.
                language: i18n::native_name(&i18n::current_locale()).into(),
                region: i18n::tr("oobe_region_default").into(),
                keyboard: i18n::tr("oobe_keyboard_default").into(),
                theme: i18n::tr("oobe_theme_default").into(),
                accent: i18n::tr("oobe_accent_default").into(),
                wallpaper: i18n::tr("oobe_wallpaper_default").into(),
                layout: i18n::tr("oobe_layout_default").into(),
                dock: i18n::tr("oobe_dock_default").into(),
                timezone: i18n::tr("oobe_tz_prague").into(),
                large_text: false,
                high_contrast: false,
                reduced_motion: false,
                screen_reader: false,
                location: false,
                diagnostics: false,
                auto_updates: true,
                gestures: true,
                tiling: false,
                favorite_terminal: true,
                favorite_files: true,
                favorite_settings: true,
                status: String::new(),
                pending_theme: None,
                theme_switched: false,
                theme_fade: 0.0,
            },
            Task::batch([immediate, delayed]),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Start => {
                self.quick = false;
                self.page = Page::License;
            }
            Message::Skip => {
                self.quick = true;
                self.page = Page::License;
            }
            Message::Back => self.go_back(),
            Message::Next => self.go_next(),
            Message::Agree(value) => self.agreed = value,
            Message::FullName(value) => self.full_name = value,
            Message::Username(value) => self.username = sanitize_username(&value),
            Message::Password(value) => self.password = value,
            Message::Confirm(value) => self.confirm = value,
            Message::TogglePassword => self.password_hidden = !self.password_hidden,
            Message::ToggleConfirm => self.confirm_hidden = !self.confirm_hidden,
            Message::Choose(Choice::Theme, value) => {
                if value != self.theme {
                    self.pending_theme = Some(value);
                    self.theme_switched = false;
                    self.theme_fade = 0.0;
                }
            }
            Message::Choose(kind, value) => self.choose(kind, value),
            Message::Toggle(kind) => self.toggle(kind),
            Message::Apply | Message::Retry => match self.start_apply() {
                Ok(()) => {
                    self.page = Page::Applying;
                    self.status = i18n::tr("oobe_applying");
                }
                Err(error) => {
                    self.page = Page::Error;
                    self.status = error;
                }
            },
            Message::Poll => self.poll(),
            Message::EnsureFullscreen => {
                if let Some(id) = self.core.main_window_id() {
                    return set_mode(id, window::Mode::Fullscreen);
                }
            }
            Message::EnterDesktop => {
                // OOBE is running in the root-owned bootstrap session.  Do
                // not exit that compositor (which produces a black screen);
                // reboot into the normal greetd/login path instead.
                // BusyBox init owns the reboot sequence.  Signalling PID 1
                // is reliable even while the root-owned OOBE compositor is
                // being torn down; spawning reboot here could leave only a
                // black framebuffer when its child process was reaped.
                let _ = Command::new("/bin/kill")
                    .args(["-TERM", "1"])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();
            }
            Message::ThemeFadeTick => return self.theme_fade_tick(),
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        self.shell()
    }

    fn subscription(&self) -> Subscription<Message> {
        if self.pending_theme.is_some() {
            cosmic::iced::time::every(Duration::from_millis(28)).map(|_| Message::ThemeFadeTick)
        } else if self.page == Page::Applying {
            cosmic::iced::time::every(Duration::from_millis(300)).map(|_| Message::Poll)
        } else {
            Subscription::none()
        }
    }
}

impl Oobe {
    fn theme_fade_tick(&mut self) -> Task<Message> {
        let Some(target) = self.pending_theme.clone() else {
            return Task::none();
        };
        if !self.theme_switched {
            self.theme_fade = (self.theme_fade + 0.12).min(1.0);
            if self.theme_fade >= 1.0 {
                self.theme_switched = true;
                self.theme = target.clone();
                let mode = match target.as_str() {
                    "Light" => "light",
                    "Automatic" => "automatic",
                    _ => "dark",
                };
                preview(&["theme", mode]);
                let selected = if target == "Light" {
                    cosmic::theme::system_light()
                } else {
                    cosmic::theme::system_dark()
                };
                return cosmic::command::set_theme(selected);
            }
        } else {
            self.theme_fade = (self.theme_fade - 0.12).max(0.0);
            if self.theme_fade <= 0.0 {
                self.pending_theme = None;
                self.theme_switched = false;
            }
        }
        Task::none()
    }

    fn go_back(&mut self) {
        self.page = match self.page {
            Page::License => Page::Welcome,
            Page::Account => Page::License,
            Page::Review if self.quick => Page::Account,
            page => Page::FULL
                .iter()
                .position(|p| *p == page)
                .and_then(|i| i.checked_sub(1))
                .map(|i| Page::FULL[i])
                .unwrap_or(Page::Welcome),
        };
    }

    fn go_next(&mut self) {
        if self.page == Page::Account && self.quick {
            self.page = Page::Review;
            return;
        }
        if let Some(index) = Page::FULL.iter().position(|page| *page == self.page) {
            if index + 1 < Page::FULL.len() {
                self.page = Page::FULL[index + 1];
            }
        }
    }

    fn choose(&mut self, kind: Choice, value: String) {
        match kind {
            Choice::Language => self.language = value,
            Choice::Region => self.region = value,
            Choice::Keyboard => self.keyboard = value,
            Choice::Theme => self.theme = value,
            Choice::Accent => self.accent = value,
            Choice::Wallpaper => {
                if let Some(filename) = wallpaper_filename(&value) {
                    preview(&["wallpaper", filename]);
                }
                self.wallpaper = value;
            }
            Choice::Layout => self.layout = value,
            Choice::Dock => self.dock = value,
            Choice::Timezone => self.timezone = value,
        }
    }

    fn toggle(&mut self, kind: Toggle) {
        let target = match kind {
            Toggle::LargeText => &mut self.large_text,
            Toggle::HighContrast => &mut self.high_contrast,
            Toggle::ReducedMotion => &mut self.reduced_motion,
            Toggle::ScreenReader => &mut self.screen_reader,
            Toggle::Location => &mut self.location,
            Toggle::Diagnostics => &mut self.diagnostics,
            Toggle::AutoUpdates => &mut self.auto_updates,
            Toggle::Gestures => &mut self.gestures,
            Toggle::Tiling => &mut self.tiling,
            Toggle::Terminal => &mut self.favorite_terminal,
            Toggle::Files => &mut self.favorite_files,
            Toggle::Settings => &mut self.favorite_settings,
        };
        *target = !*target;
    }

    fn valid_account(&self) -> bool {
        !self.full_name.trim().is_empty()
            && self.username.len() >= 2
            && self.password.len() >= 6
            && self.password == self.confirm
    }

    fn shell(&self) -> Element<'_, Message> {
        let current = Page::FULL
            .iter()
            .position(|p| *p == self.page)
            .unwrap_or(Page::FULL.len() - 1);
        let mut steps = widget::column::with_capacity(Page::FULL.len() + 2)
            .spacing(5)
            .push(
                widget::row::with_capacity(2)
                    .spacing(10)
                    .align_y(Alignment::Center)
                    .push(widget::icon::from_name("com.man.Utilities").size(42))
                    .push(widget::text::heading(i18n::tr("oobe_setup"))),
            )
            .push(widget::space::vertical().height(10));
        for (index, page) in Page::FULL.iter().enumerate() {
            if index > 2 && self.quick {
                continue;
            }
            let row = widget::row::with_capacity(2)
                .spacing(10)
                .align_y(Alignment::Center)
                .push(widget::icon::from_name(page.icon()).size(20))
                .push(widget::text::body(page.title_tr()));
            steps = steps.push(
                widget::container(row)
                    .class(if *page == self.page {
                        theme::Container::Card
                    } else {
                        theme::Container::Transparent
                    })
                    .padding([7, 9]),
            );
        }
        let sidebar = widget::container(widget::scrollable(steps).height(Length::Fill))
            .class(theme::Container::Secondary)
            .padding(22)
            .width(Length::Fixed(285.0))
            .height(Length::Fill);

        let progress = if matches!(self.page, Page::Applying | Page::Done) {
            1.0
        } else {
            (current + 1) as f32 / Page::FULL.len() as f32
        };
        let content = widget::column::with_capacity(4)
            .spacing(18)
            .push(
                widget::progress_bar::linear::Linear::new()
                    .progress(progress)
                    .width(Length::Fill),
            )
            .push(widget::scrollable(self.page_content()).height(Length::Fill))
            .push(self.navigation());
        let main = widget::container(content)
            .padding([30, 46])
            .width(Length::Fill)
            .height(Length::Fill);
        let base: Element<'_, Message> =
            widget::container(widget::row::with_capacity(2).push(sidebar).push(main))
                .class(theme::Container::WindowBackground)
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        if self.theme_fade <= 0.0 {
            return base;
        }
        let alpha = self.theme_fade.clamp(0.0, 1.0);
        let light = self.pending_theme.as_deref() == Some("Light");
        let overlay = widget::container(widget::space::vertical())
            .class(theme::Container::custom(move |_| {
                widget::container::Style {
                    background: Some(Background::Color(Color {
                        r: if light { 1.0 } else { 0.0 },
                        g: if light { 1.0 } else { 0.0 },
                        b: if light { 1.0 } else { 0.0 },
                        a: alpha,
                    })),
                    ..Default::default()
                }
            }))
            .width(Length::Fill)
            .height(Length::Fill);
        cosmic::iced::widget::stack![base, overlay].into()
    }

    fn page_content(&self) -> Element<'_, Message> {
        let header = widget::column::with_capacity(3)
            .spacing(8)
            .align_x(Alignment::Center)
            .push(widget::icon::from_name(self.page.icon()).size(72))
            .push(widget::text::title1(self.page.title_tr()))
            .push(widget::text::body(match self.page {
                Page::Welcome => i18n::tr("oobe_welcome_sub"),
                Page::License => i18n::tr("oobe_license"),
                Page::Account => i18n::tr("oobe_account_sub"),
                Page::Review => i18n::tr("oobe_review_sub"),
                Page::Applying => i18n::tr("oobe_applying_sub2"),
                Page::Done => i18n::tr("oobe_done_sub"),
                Page::Error => i18n::tr("oobe_no_completion"),
                _ => i18n::tr("oobe_make_home"),
            }));
        let body: Element<'_, Message> = match self.page {
            Page::Welcome => widget::column::with_capacity(6)
                .spacing(16)
                .align_x(Alignment::Center)
                .push(widget::text::heading(i18n::tr("oobe_make_yours")))
                .push(widget::text::body(i18n::tr("oobe_full_tour")))
                .push(
                    widget::button::suggested(i18n::tr("oobe_start_full")).on_press(Message::Start),
                )
                .push(
                    widget::button::standard(i18n::tr("oobe_skip_personal"))
                        .on_press(Message::Skip),
                )
                .push(widget::text::caption(i18n::tr("oobe_quick_note")))
                .into(),
            Page::License => widget::column::with_capacity(3)
                .spacing(14)
                .push(
                    widget::container(
                        widget::scrollable(widget::text::body(LICENSE))
                            .height(Length::Fixed(250.0)),
                    )
                    .class(theme::Container::Secondary)
                    .padding(18)
                    .width(Length::Fill),
                )
                .push(self.toggle_card(
                    "emblem-ok-symbolic",
                    i18n::tr("oobe_agree"),
                    self.agreed,
                    Message::Agree(!self.agreed),
                ))
                .into(),
            Page::Account => {
                let note = if !self.password.is_empty() && self.password != self.confirm {
                    i18n::tr("oobe_passwords_mismatch")
                } else {
                    i18n::tr("oobe_password_note")
                };
                widget::column::with_capacity(6)
                    .spacing(12)
                    .push(
                        widget::text_input(i18n::tr("oobe_full_name"), &self.full_name)
                            .label(i18n::tr("oobe_full_name"))
                            .on_input(Message::FullName),
                    )
                    .push(
                        widget::text_input("username", &self.username)
                            .label(i18n::tr("oobe_account_name"))
                            .on_input(Message::Username),
                    )
                    .push(
                        widget::secure_input(
                            i18n::tr("oobe_password"),
                            &self.password,
                            Some(Message::TogglePassword),
                            self.password_hidden,
                        )
                        .label(i18n::tr("oobe_password"))
                        .on_input(Message::Password),
                    )
                    .push(
                        widget::secure_input(
                            i18n::tr("oobe_confirm_password"),
                            &self.confirm,
                            Some(Message::ToggleConfirm),
                            self.confirm_hidden,
                        )
                        .label(i18n::tr("oobe_confirm_password"))
                        .on_input(Message::Confirm),
                    )
                    .push(widget::text::caption(note))
                    .into()
            }
            Page::Language => self.options(
                Choice::Language,
                &self.language,
                &[
                    (
                        "preferences-desktop-locale",
                        i18n::tr("oobe_lang_en_us"),
                        i18n::tr("oobe_lang_en_us_desc"),
                    ),
                    (
                        "preferences-desktop-locale",
                        i18n::tr("oobe_lang_en_gb"),
                        i18n::tr("oobe_lang_en_gb_desc"),
                    ),
                    (
                        "preferences-desktop-locale",
                        i18n::tr("oobe_lang_de"),
                        i18n::tr("oobe_lang_de_desc"),
                    ),
                    (
                        "preferences-desktop-locale",
                        i18n::tr("oobe_lang_cs"),
                        i18n::tr("oobe_lang_cs_desc"),
                    ),
                ],
            ),
            Page::Region => self.options(
                Choice::Region,
                &self.region,
                &[
                    (
                        "mark-location-symbolic",
                        i18n::tr("oobe_region_us"),
                        i18n::tr("oobe_region_us_desc"),
                    ),
                    (
                        "mark-location-symbolic",
                        i18n::tr("oobe_region_uk"),
                        i18n::tr("oobe_region_uk_desc"),
                    ),
                    (
                        "mark-location-symbolic",
                        i18n::tr("oobe_region_cz"),
                        i18n::tr("oobe_region_cz_desc"),
                    ),
                    (
                        "mark-location-symbolic",
                        i18n::tr("oobe_region_de"),
                        i18n::tr("oobe_region_de_desc"),
                    ),
                ],
            ),
            Page::Keyboard => self.options(
                Choice::Keyboard,
                &self.keyboard,
                &[
                    (
                        "preferences-desktop-keyboard",
                        i18n::tr("oobe_kb_us"),
                        i18n::tr("oobe_kb_us_desc"),
                    ),
                    (
                        "preferences-desktop-keyboard",
                        i18n::tr("oobe_kb_uk"),
                        i18n::tr("oobe_kb_uk_desc"),
                    ),
                    (
                        "preferences-desktop-keyboard",
                        i18n::tr("oobe_kb_cz"),
                        i18n::tr("oobe_kb_cz_desc"),
                    ),
                    (
                        "preferences-desktop-keyboard",
                        i18n::tr("oobe_kb_de"),
                        i18n::tr("oobe_kb_de_desc"),
                    ),
                ],
            ),
            Page::Theme => self.options(
                Choice::Theme,
                &self.theme,
                &[
                    (
                        "weather-clear-symbolic",
                        i18n::tr("oobe_theme_light"),
                        i18n::tr("oobe_theme_light_desc"),
                    ),
                    (
                        "weather-clear-night-symbolic",
                        i18n::tr("oobe_theme_dark"),
                        i18n::tr("oobe_theme_dark_desc"),
                    ),
                    (
                        "preferences-system-time",
                        i18n::tr("oobe_theme_auto"),
                        i18n::tr("oobe_theme_auto_desc"),
                    ),
                ],
            ),
            Page::Accent => self.options(
                Choice::Accent,
                &self.accent,
                &[
                    (
                        "applications-graphics",
                        i18n::tr("oobe_accent_blue"),
                        i18n::tr("oobe_accent_blue_desc"),
                    ),
                    (
                        "applications-graphics",
                        i18n::tr("oobe_accent_orange"),
                        i18n::tr("oobe_accent_orange_desc"),
                    ),
                    (
                        "applications-graphics",
                        i18n::tr("oobe_accent_green"),
                        i18n::tr("oobe_accent_green_desc"),
                    ),
                    (
                        "applications-graphics",
                        i18n::tr("oobe_accent_violet"),
                        i18n::tr("oobe_accent_violet_desc"),
                    ),
                ],
            ),
            Page::Wallpaper => self.wallpaper_options(),
            Page::Layout => self.options(
                Choice::Layout,
                &self.layout,
                &[
                    (
                        "view-grid-symbolic",
                        i18n::tr("oobe_layout_balanced"),
                        i18n::tr("oobe_layout_balanced_desc"),
                    ),
                    (
                        "view-list-symbolic",
                        i18n::tr("oobe_layout_compact"),
                        i18n::tr("oobe_layout_compact_desc"),
                    ),
                    (
                        "view-fullscreen-symbolic",
                        i18n::tr("oobe_layout_focused"),
                        i18n::tr("oobe_layout_focused_desc"),
                    ),
                ],
            ),
            Page::Dock => self.options(
                Choice::Dock,
                &self.dock,
                &[
                    (
                        "go-bottom-symbolic",
                        i18n::tr("oobe_dock_auto"),
                        i18n::tr("oobe_dock_auto_desc"),
                    ),
                    (
                        "go-bottom-symbolic",
                        i18n::tr("oobe_dock_visible"),
                        i18n::tr("oobe_dock_visible_desc"),
                    ),
                    (
                        "go-bottom-symbolic",
                        i18n::tr("oobe_dock_hidden"),
                        i18n::tr("oobe_dock_hidden_desc"),
                    ),
                ],
            ),
            Page::Accessibility => self.toggles(&[
                (
                    "preferences-desktop-font",
                    i18n::tr("oobe_large_text"),
                    i18n::tr("oobe_large_text_desc"),
                    self.large_text,
                    Toggle::LargeText,
                ),
                (
                    "preferences-desktop-appearance",
                    i18n::tr("oobe_high_contrast"),
                    i18n::tr("oobe_high_contrast_desc"),
                    self.high_contrast,
                    Toggle::HighContrast,
                ),
                (
                    "media-playback-pause-symbolic",
                    i18n::tr("oobe_reduce_motion"),
                    i18n::tr("oobe_reduce_motion_desc"),
                    self.reduced_motion,
                    Toggle::ReducedMotion,
                ),
                (
                    "preferences-desktop-accessibility",
                    i18n::tr("oobe_screen_reader"),
                    i18n::tr("oobe_screen_reader_desc"),
                    self.screen_reader,
                    Toggle::ScreenReader,
                ),
            ]),
            Page::Privacy => self.toggles(&[
                (
                    "mark-location-symbolic",
                    i18n::tr("oobe_location"),
                    i18n::tr("oobe_location_desc"),
                    self.location,
                    Toggle::Location,
                ),
                (
                    "utilities-system-monitor",
                    i18n::tr("oobe_diagnostics"),
                    i18n::tr("oobe_diagnostics_desc"),
                    self.diagnostics,
                    Toggle::Diagnostics,
                ),
            ]),
            Page::Updates => self.toggles(&[(
                "preferences-system-updates",
                i18n::tr("oobe_auto_updates"),
                i18n::tr("oobe_auto_updates_desc"),
                self.auto_updates,
                Toggle::AutoUpdates,
            )]),
            Page::Timezone => self.options(
                Choice::Timezone,
                &self.timezone,
                &[
                    (
                        "preferences-system-time",
                        i18n::tr("oobe_tz_prague"),
                        i18n::tr("oobe_tz_prague_desc"),
                    ),
                    (
                        "preferences-system-time",
                        i18n::tr("oobe_tz_london"),
                        i18n::tr("oobe_tz_london_desc"),
                    ),
                    (
                        "preferences-system-time",
                        i18n::tr("oobe_tz_ny"),
                        i18n::tr("oobe_tz_ny_desc"),
                    ),
                    (
                        "preferences-system-time",
                        i18n::tr("oobe_tz_tokyo"),
                        i18n::tr("oobe_tz_tokyo_desc"),
                    ),
                ],
            ),
            Page::Gestures => self.toggles(&[
                (
                    "preferences-touchpad",
                    i18n::tr("oobe_touchpad_gestures"),
                    i18n::tr("oobe_touchpad_gestures_desc"),
                    self.gestures,
                    Toggle::Gestures,
                ),
                (
                    "view-grid-symbolic",
                    i18n::tr("oobe_tiling"),
                    i18n::tr("oobe_tiling_desc"),
                    self.tiling,
                    Toggle::Tiling,
                ),
            ]),
            Page::Apps => self.toggles(&[
                (
                    "com.system76.CosmicTerm",
                    i18n::tr("oobe_terminal"),
                    i18n::tr("oobe_terminal_desc"),
                    self.favorite_terminal,
                    Toggle::Terminal,
                ),
                (
                    "com.system76.CosmicFiles",
                    i18n::tr("oobe_files"),
                    i18n::tr("oobe_files_desc"),
                    self.favorite_files,
                    Toggle::Files,
                ),
                (
                    "com.system76.CosmicSettings",
                    i18n::tr("oobe_settings"),
                    i18n::tr("oobe_settings_desc"),
                    self.favorite_settings,
                    Toggle::Settings,
                ),
            ]),
            Page::Review => self.review(),
            Page::Applying => widget::column::with_capacity(4)
                .spacing(22)
                .align_x(Alignment::Center)
                .push(
                    widget::progress_bar::linear::Linear::new()
                        .progress(0.66)
                        .width(Length::Fixed(520.0)),
                )
                .push(widget::text::heading(&self.status))
                .push(widget::text::body(i18n::tr("oobe_applying_sub")))
                .into(),
            Page::Done => widget::column::with_capacity(4)
                .spacing(18)
                .align_x(Alignment::Center)
                .push(widget::icon::from_name("emblem-ok-symbolic").size(96))
                .push(widget::text::heading(
                    i18n::tr("oobe_done_welcome").replace("{0}", self.full_name.trim()),
                ))
                .push(widget::text::body(i18n::tr("oobe_change_later")))
                .push(
                    widget::button::suggested(i18n::tr("oobe_enter_desktop"))
                        .on_press(Message::EnterDesktop),
                )
                .into(),
            Page::Error => widget::column::with_capacity(3)
                .spacing(18)
                .align_x(Alignment::Center)
                .push(widget::text::body(&self.status))
                .push(
                    widget::button::suggested(i18n::tr("oobe_try_again")).on_press(Message::Retry),
                )
                .into(),
        };
        widget::column::with_capacity(2)
            .spacing(24)
            .align_x(Alignment::Center)
            .push(header)
            .push(widget::container(body).max_width(820).width(Length::Fill))
            .into()
    }

    fn navigation(&self) -> Element<'_, Message> {
        if matches!(
            self.page,
            Page::Welcome | Page::Applying | Page::Done | Page::Error
        ) {
            return widget::space::vertical().height(1).into();
        }
        let enabled = match self.page {
            Page::License => self.agreed,
            Page::Account => self.valid_account(),
            _ => true,
        };
        let action = if self.page == Page::Review {
            Message::Apply
        } else {
            Message::Next
        };
        let label = if self.page == Page::Review {
            i18n::tr("oobe_apply")
        } else {
            i18n::tr("continue")
        };
        widget::row::with_capacity(3)
            .spacing(12)
            .push(widget::button::standard(i18n::tr("back")).on_press(Message::Back))
            .push(widget::space::horizontal())
            .push(widget::button::suggested(label).on_press_maybe(enabled.then_some(action)))
            .into()
    }

    fn options(
        &self,
        kind: Choice,
        selected: &str,
        choices: &[(&'static str, String, String)],
    ) -> Element<'_, Message> {
        let mut grid = widget::column::with_capacity(choices.len()).spacing(10);
        for (icon, title, description) in choices {
            let content = widget::row::with_capacity(3)
                .spacing(15)
                .align_y(Alignment::Center)
                .push(widget::icon::from_name(*icon).size(36))
                .push(
                    widget::column::with_capacity(2)
                        .spacing(3)
                        .push(widget::text::heading(title.clone()))
                        .push(widget::text::caption(description.clone())),
                )
                .push(widget::space::horizontal());
            grid = grid.push(
                widget::button::custom(content)
                    .class(if selected == title.as_str() {
                        theme::Button::Suggested
                    } else {
                        theme::Button::Standard
                    })
                    .padding(14)
                    .width(Length::Fill)
                    .on_press(Message::Choose(kind, title.clone())),
            );
        }
        grid.into()
    }

    fn wallpaper_options(&self) -> Element<'_, Message> {
        let wallpapers: [(String, &str, String); 4] = [
            (
                i18n::tr("oobe_wallpaper_default"),
                "orion_nebula_nasa_heic0601a.jpg",
                i18n::tr("oobe_wallpaper_default_desc"),
            ),
            (
                i18n::tr("oobe_wallpaper_webb"),
                "webb-inspired-wallpaper-system76.jpg",
                i18n::tr("oobe_wallpaper_webb_desc"),
            ),
            (
                i18n::tr("oobe_wallpaper_earth"),
                "otherworldly_earth_nasa_ISS064-E-29444.jpg",
                i18n::tr("oobe_wallpaper_earth_desc"),
            ),
            (
                i18n::tr("oobe_wallpaper_moons"),
                "round_moons_nasa.jpg",
                i18n::tr("oobe_wallpaper_moons_desc"),
            ),
        ];
        let mut rows = widget::column::with_capacity(2).spacing(14);
        for pair in wallpapers.chunks(2) {
            let mut row = widget::row::with_capacity(2).spacing(14);
            for (title, filename, description) in pair {
                let path = format!("/usr/share/backgrounds/cosmic/{filename}");
                let preview = widget::image(widget::image::Handle::from_path(path))
                    .width(Length::Fill)
                    .height(Length::Fixed(132.0))
                    .content_fit(cosmic::iced::ContentFit::Cover);
                let content = widget::column::with_capacity(3)
                    .spacing(8)
                    .push(preview)
                    .push(widget::text::heading(title.clone()))
                    .push(widget::text::caption(description.clone()));
                row = row.push(
                    widget::button::custom(content)
                        .class(if self.wallpaper.as_str() == title.as_str() {
                            theme::Button::Suggested
                        } else {
                            theme::Button::Standard
                        })
                        .padding(10)
                        .width(Length::FillPortion(1))
                        .on_press(Message::Choose(Choice::Wallpaper, (*title).clone())),
                );
            }
            rows = rows.push(row);
        }
        rows.into()
    }

    fn toggles(
        &self,
        choices: &[(&'static str, String, String, bool, Toggle)],
    ) -> Element<'_, Message> {
        let mut column = widget::column::with_capacity(choices.len()).spacing(10);
        for (icon, title, description, value, kind) in choices {
            column =
                column.push(self.toggle_card(icon, title.clone(), *value, Message::Toggle(*kind)));
            column = column.push(widget::text::caption(description.clone()));
        }
        column.into()
    }

    fn toggle_card(
        &self,
        icon: &str,
        title: String,
        value: bool,
        message: Message,
    ) -> Element<'_, Message> {
        let content = widget::row::with_capacity(3)
            .spacing(14)
            .align_y(Alignment::Center)
            .push(widget::icon::from_name(icon).size(32))
            .push(widget::text::heading(title))
            .push(widget::space::horizontal())
            .push(widget::text::body(if value {
                i18n::tr("on_state")
            } else {
                i18n::tr("off_state")
            }));
        widget::button::custom(content)
            .class(if value {
                theme::Button::Suggested
            } else {
                theme::Button::Standard
            })
            .padding(14)
            .width(Length::Fill)
            .on_press(message)
            .into()
    }

    fn review(&self) -> Element<'_, Message> {
        let personalization = if self.quick {
            i18n::tr("oobe_skipped").into()
        } else {
            format!(
                "{} · {} · {} · {}",
                self.theme, self.wallpaper, self.layout, self.dock
            )
        };
        widget::column::with_capacity(6)
            .spacing(10)
            .push(self.summary(
                "system-users",
                i18n::tr("oobe_account"),
                format!("{} (@{})", self.full_name.trim(), self.username),
            ))
            .push(self.summary(
                "preferences-desktop-locale",
                i18n::tr("oobe_language_short"),
                format!("{} · {}", self.language, self.region),
            ))
            .push(self.summary(
                "preferences-desktop-keyboard",
                i18n::tr("oobe_keyboard"),
                self.keyboard.clone(),
            ))
            .push(self.summary(
                "preferences-desktop-appearance",
                i18n::tr("oobe_personalization"),
                personalization,
            ))
            .push(self.summary(
                "preferences-system-time",
                i18n::tr("oobe_timezone_short"),
                self.timezone.clone(),
            ))
            .push(self.summary(
                "preferences-system-privacy",
                i18n::tr("oobe_privacy"),
                if self.diagnostics {
                    i18n::tr("oobe_diagnostics_enabled").into()
                } else {
                    i18n::tr("oobe_diagnostics_disabled").into()
                },
            ))
            .into()
    }

    fn summary(&self, icon: &str, title: String, value: String) -> Element<'_, Message> {
        widget::container(
            widget::row::with_capacity(3)
                .spacing(14)
                .align_y(Alignment::Center)
                .push(widget::icon::from_name(icon).size(30))
                .push(widget::text::heading(title))
                .push(widget::space::horizontal())
                .push(widget::text::body(value)),
        )
        .class(theme::Container::Secondary)
        .padding(14)
        .width(Length::Fill)
        .into()
    }

    fn start_apply(&self) -> Result<(), String> {
        let mut file = fs::File::create(REQUEST)
            .map_err(|e| format!("Could not create setup request: {e}"))?;
        fs::set_permissions(REQUEST, fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("Could not secure setup request: {e}"))?;
        // UI labels are translated, but the privileged backend must receive
        // stable IDs. Comparing against translated option keys here keeps
        // every locale independent from the shell implementation.
        let language = selection_id(
            &self.language,
            &[
                ("oobe_lang_en_us", "en_US"),
                ("oobe_lang_en_gb", "en_GB"),
                ("oobe_lang_de", "de_DE"),
                ("oobe_lang_cs", "cs_CZ"),
            ],
            &i18n::current_locale(),
        );
        let region = selection_id(
            &self.region,
            &[
                ("oobe_region_us", "US"),
                ("oobe_region_uk", "GB"),
                ("oobe_region_cz", "CZ"),
                ("oobe_region_de", "DE"),
            ],
            "US",
        );
        let keyboard = selection_id(
            &self.keyboard,
            &[
                ("oobe_kb_us", "us"),
                ("oobe_kb_uk", "gb"),
                ("oobe_kb_cz", "cz"),
                ("oobe_kb_de", "de"),
            ],
            "us",
        );
        let theme = selection_id(
            &self.theme,
            &[
                ("oobe_theme_light", "light"),
                ("oobe_theme_dark", "dark"),
                ("oobe_theme_auto", "automatic"),
            ],
            "dark",
        );
        let wallpaper = selection_id(
            &self.wallpaper,
            &[
                ("oobe_wallpaper_default", "orion"),
                ("oobe_wallpaper_webb", "webb"),
                ("oobe_wallpaper_earth", "earth"),
                ("oobe_wallpaper_moons", "moons"),
            ],
            "orion",
        );
        let layout = selection_id(
            &self.layout,
            &[
                ("oobe_layout_balanced", "balanced"),
                ("oobe_layout_compact", "compact"),
                ("oobe_layout_focused", "focused"),
            ],
            "balanced",
        );
        let dock = selection_id(
            &self.dock,
            &[
                ("oobe_dock_auto", "auto"),
                ("oobe_dock_visible", "visible"),
                ("oobe_dock_hidden", "hidden"),
            ],
            "auto",
        );
        let timezone = selection_id(
            &self.timezone,
            &[
                ("oobe_tz_prague", "Europe/Prague"),
                ("oobe_tz_london", "Europe/London"),
                ("oobe_tz_ny", "America/New_York"),
                ("oobe_tz_tokyo", "Asia/Tokyo"),
            ],
            "Europe/Prague",
        );
        let lines = [
            ("full_name", self.full_name.as_str()),
            ("username", self.username.as_str()),
            ("personalize", bool_text(!self.quick)),
            ("language", language.as_str()),
            ("region", region.as_str()),
            ("keyboard", keyboard.as_str()),
            ("theme", theme.as_str()),
            ("accent", self.accent.as_str()),
            ("wallpaper", wallpaper.as_str()),
            ("layout", layout.as_str()),
            ("dock", dock.as_str()),
            ("timezone", timezone.as_str()),
            ("large_text", bool_text(self.large_text)),
            ("high_contrast", bool_text(self.high_contrast)),
            ("reduced_motion", bool_text(self.reduced_motion)),
            ("screen_reader", bool_text(self.screen_reader)),
            ("location", bool_text(self.location)),
            ("diagnostics", bool_text(self.diagnostics)),
            ("auto_updates", bool_text(self.auto_updates)),
            ("gestures", bool_text(self.gestures)),
            ("tiling", bool_text(self.tiling)),
            ("favorite_terminal", bool_text(self.favorite_terminal)),
            ("favorite_files", bool_text(self.favorite_files)),
            ("favorite_settings", bool_text(self.favorite_settings)),
        ];
        for (key, value) in lines {
            writeln!(file, "{}={}", key, value.replace(['\n', '\r', '='], " "))
                .map_err(|e| e.to_string())?;
        }
        drop(file);
        let mut password_file = fs::File::create(PASSWORD_FILE)
            .map_err(|e| format!("Could not create password request: {e}"))?;
        fs::set_permissions(PASSWORD_FILE, fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("Could not secure password request: {e}"))?;
        password_file
            .write_all(self.password.as_bytes())
            .map_err(|e| format!("Could not write password request: {e}"))?;
        drop(password_file);
        let _ = fs::remove_file(STATUS);
        Command::new("/usr/sbin/man-oobe-apply")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Could not start setup service: {e}"))?;
        Ok(())
    }

    fn poll(&mut self) {
        let Ok(status) = fs::read_to_string(STATUS) else {
            return;
        };
        let mut fields = status.trim().splitn(2, '|');
        match fields.next().unwrap_or("") {
            "working" => {
                let value = fields.next().unwrap_or("@oobe_applying");
                self.status = value
                    .strip_prefix('@')
                    .map(i18n::tr)
                    .unwrap_or_else(|| value.into());
            }
            "complete" => {
                self.status = i18n::tr("oobe_complete").into();
                self.password.clear();
                self.confirm.clear();
                self.page = Page::Done;
            }
            "failed" => {
                let value = fields.next().unwrap_or("@oobe_error_sub");
                self.status = value
                    .strip_prefix('@')
                    .map(i18n::tr)
                    .unwrap_or_else(|| value.into());
                self.page = Page::Error;
            }
            _ => {}
        }
    }
}

fn sanitize_username(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_' || *c == '-')
        .take(24)
        .collect()
}

fn bool_text(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn wallpaper_filename(name: &str) -> Option<&'static str> {
    let wallpapers: [(String, &'static str); 4] = [
        (
            i18n::tr("oobe_wallpaper_default"),
            "orion_nebula_nasa_heic0601a.jpg",
        ),
        (
            i18n::tr("oobe_wallpaper_webb"),
            "webb-inspired-wallpaper-system76.jpg",
        ),
        (
            i18n::tr("oobe_wallpaper_earth"),
            "otherworldly_earth_nasa_ISS064-E-29444.jpg",
        ),
        (i18n::tr("oobe_wallpaper_moons"), "round_moons_nasa.jpg"),
    ];
    wallpapers.iter().find(|(n, _)| n == name).map(|(_, f)| *f)
}

fn selection_id(value: &str, choices: &[(&str, &str)], fallback: &str) -> String {
    choices
        .iter()
        .find(|(translation_key, _)| i18n::tr(translation_key) == value)
        .map(|(_, id)| (*id).to_string())
        .unwrap_or_else(|| fallback.to_string())
}

fn preview(arguments: &[&str]) {
    let _ = Command::new("/usr/bin/man-oobe-preview")
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}
