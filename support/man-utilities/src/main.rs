use cosmic::app::{Core, Settings, Task};
use cosmic::iced::window::{self, set_mode};
use cosmic::iced::{Alignment, Length, Limits, Subscription};
use cosmic::{Application, Element, executor, theme, widget};
use std::fs::{self, File};
use std::process::{Command, Stdio};
use std::time::Duration;

mod i18n;
mod guide;
mod network;
mod oobe;

const INSTALL_STATUS: &str = "/run/man-installer.status";
const INSTALL_PID: &str = "/run/man-installer.pid";
const DISK_STATUS: &str = "/run/man-diskulator.status";
const DISK_PID: &str = "/run/man-diskulator.pid";
const LICENSE: &str = "MAN SOFTWARE NOTICE\n\nMAN-authored components are proprietary: Copyright (c) 2026 MAN project contributors. All Rights Reserved. This notice does not apply to included open-source software, which retains its own copyright notices and license terms. Third-party notices and release compliance information are available in /usr/share/doc/MAN.\n\nTHE SOFTWARE IS PROVIDED WITHOUT WARRANTY OF ANY KIND. You are responsible for backups and for choosing the correct installation disk. Installation and Diskulator erase operations permanently destroy data on the selected device.\n\nBy choosing Agree, you confirm that you are authorized to install this software, accept the applicable third-party terms, and understand that the selected destination disk will be erased.";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    i18n::init();
    let oobe = std::env::args().any(|argument| argument == "--oobe")
        || std::env::args()
            .next()
            .is_some_and(|program| program.ends_with("man-oobe"));
    if oobe {
        return oobe::run();
    }
    let network = std::env::args().any(|argument| argument == "--network")
        || std::env::args()
            .next()
            .is_some_and(|program| program.ends_with("man-network"));
    if network {
        return network::run();
    }
    let guide = env!("CARGO_BIN_NAME") == "man-guide"
        || std::env::args().any(|argument| argument == "--guide")
        || std::env::args()
            .next()
            .is_some_and(|program| program.ends_with("man-guide"));
    if guide {
        return guide::run();
    }
    let diskulator = std::env::args().any(|argument| argument == "--diskulator")
        || std::env::args()
            .next()
            .is_some_and(|program| program.ends_with("man-diskulator"));
    if diskulator {
        let settings = Settings::default()
            .size(cosmic::iced::Size::new(1000.0, 680.0))
            .size_limits(Limits::NONE.min_width(760.0).min_height(540.0));
        cosmic::app::run::<DiskulatorApp>(settings, ())?;
    } else {
        let settings = Settings::default()
            .size_limits(Limits::NONE)
            .exit_on_close(false)
            .client_decorations(false);
        cosmic::app::run::<Utilities>(settings, ())?;
    }
    Ok(())
}

#[derive(Clone, Debug)]
enum Message {
    Home,
    LanguageSelected(String),
    LanguageConfirmed,
    Installer,
    InstallerLicense,
    LicenseDisagree,
    LicenseAgree,
    InstallerConfirm,
    Diskulator,
    BackToDiskList,
    Terminal,
    SelectDisk(String),
    BeginInstall,
    ChooseDiskOperation(DiskOperation),
    BeginDiskOperation,
    PollJob,
    CancelJob,
    EnsureFullscreen,
    RebootTick,
    RebootNow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    Language,
    Home,
    InstallerWelcome,
    License,
    InstallerDisk,
    InstallerConfirm,
    Installing,
    InstallComplete,
    JobError,
    Diskulator,
    DiskConfirm,
    DiskWorking,
    DiskComplete,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DiskOperation {
    PartitionMan,
    EraseFat32,
    EraseExt2,
    FirstAid,
}

impl DiskOperation {
    fn title(self) -> String {
        match self {
            Self::PartitionMan => i18n::tr("partition_man"),
            Self::EraseFat32 => i18n::tr("erase_fat32"),
            Self::EraseExt2 => i18n::tr("erase_ext2"),
            Self::FirstAid => i18n::tr("first_aid"),
        }
    }
    fn description(self) -> String {
        match self {
            Self::PartitionMan => i18n::tr("partition_man_desc"),
            Self::EraseFat32 => i18n::tr("erase_fat32_desc"),
            Self::EraseExt2 => i18n::tr("erase_ext2_desc"),
            Self::FirstAid => i18n::tr("first_aid_desc"),
        }
    }
    fn argument(self) -> &'static str {
        match self {
            Self::PartitionMan => "partition-man",
            Self::EraseFat32 => "erase-fat32",
            Self::EraseExt2 => "erase-ext2",
            Self::FirstAid => "first-aid",
        }
    }
    fn destructive(self) -> bool {
        self != Self::FirstAid
    }
}

#[derive(Clone, Debug)]
struct Partition {
    path: String,
    bytes: u64,
    size: String,
    filesystem: String,
    mountpoint: String,
}

#[derive(Clone, Debug)]
struct Disk {
    path: String,
    name: String,
    size: String,
    bytes: u64,
    removable: bool,
    mounted: bool,
    partitions: Vec<Partition>,
}

struct Utilities {
    core: Core,
    page: Page,
    language: String,
    disks: Vec<Disk>,
    selected: Option<String>,
    operation: Option<DiskOperation>,
    progress: f32,
    status: String,
    diskulator_mode: bool,
    reboot_seconds: u8,
}

impl Application for Utilities {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "com.man.Utilities";

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
        let language = i18n::current_locale();
        let initial_page = if !i18n::has_language_preference() {
            Page::Language
        } else {
            Page::Home
        };
        (
            Self {
                core,
                page: initial_page,
                language,
                disks: scan_disks(),
                selected: None,
                operation: None,
                progress: 0.0,
                status: String::new(),
                diskulator_mode: false,
                reboot_seconds: 59,
            },
            Task::batch([immediate, delayed]),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Home => {
                self.page = if self.diskulator_mode {
                    Page::Diskulator
                } else {
                    Page::Home
                };
                self.operation = None;
            }
            Message::LanguageSelected(locale) => {
                self.language = locale.clone();
                i18n::save_language(&self.language);
                // Also persist the system locale so that COSMIC, shell, and
                // all other glibc/gettext-based components pick up the new
                // language on the next session restart.
                let locale_line = format!(
                    "LANG={}.UTF-8\nLC_ALL={}.UTF-8\n",
                    &self.language, &self.language
                );
                let _ = std::fs::write("/etc/default/locale", locale_line);
            }
            Message::LanguageConfirmed => {
                // The choice is already persisted by LanguageSelected. Keep
                // this process alive and continue to the installer home page.
                // Restarting the recovery compositor here can recreate the
                // utility before the transient preference is visible, which
                // traps the user in the language chooser.
                self.page = Page::Home;
            }
            Message::Installer => self.page = Page::InstallerWelcome,
            Message::InstallerLicense => self.page = Page::License,
            Message::LicenseDisagree => self.page = Page::InstallerWelcome,
            Message::LicenseAgree => {
                self.disks = scan_disks();
                self.selected = None;
                self.page = Page::InstallerDisk;
            }
            Message::InstallerConfirm => self.page = Page::InstallerConfirm,
            Message::Diskulator => {
                if self.diskulator_mode {
                    self.disks = scan_disks();
                    self.selected = self.disks.first().map(|disk| disk.path.clone());
                    self.operation = None;
                    self.page = Page::Diskulator;
                } else {
                    let _ = Command::new("/usr/bin/man-diskulator").spawn();
                }
            }
            Message::BackToDiskList => {
                self.disks = scan_disks();
                if self.selected_disk().is_none() {
                    self.selected = self.disks.first().map(|disk| disk.path.clone());
                }
                self.operation = None;
                self.page = Page::Diskulator;
            }
            Message::Terminal => {
                let _ = Command::new("/usr/bin/cosmic-term").spawn();
            }
            Message::SelectDisk(path) => self.selected = Some(path),
            Message::BeginInstall => {
                if let Some(disk) = self.selected.clone() {
                    self.progress = 1.0;
                    self.status = i18n::tr("preparing_dest");
                    match start_backend(
                        "/usr/sbin/man-install",
                        &["--yes", &disk],
                        INSTALL_STATUS,
                        INSTALL_PID,
                        "/var/log/man-installer.log",
                    ) {
                        Ok(()) => self.page = Page::Installing,
                        Err(error) => {
                            self.status = error;
                            self.page = Page::JobError;
                        }
                    }
                }
            }
            Message::ChooseDiskOperation(operation) => {
                self.operation = Some(operation);
                self.page = Page::DiskConfirm;
            }
            Message::BeginDiskOperation => {
                if let (Some(disk), Some(operation)) = (self.selected.clone(), self.operation) {
                    self.progress = 1.0;
                    self.status = i18n::tr("preparing_disk");
                    match start_backend(
                        "/usr/sbin/man-diskutil",
                        &[operation.argument(), &disk],
                        DISK_STATUS,
                        DISK_PID,
                        "/var/log/man-diskulator.log",
                    ) {
                        Ok(()) => self.page = Page::DiskWorking,
                        Err(error) => {
                            self.status = error;
                            self.page = Page::JobError;
                        }
                    }
                }
            }
            Message::PollJob => {
                let path = if self.page == Page::Installing {
                    INSTALL_STATUS
                } else {
                    DISK_STATUS
                };
                if let Some((progress, status)) = read_status(path) {
                    self.status = status;
                    if progress < 0 {
                        self.page = Page::JobError;
                    } else {
                        self.progress = progress as f32;
                        if progress >= 100 {
                            self.page = if self.page == Page::Installing {
                                self.reboot_seconds = 59;
                                Page::InstallComplete
                            } else {
                                self.disks = scan_disks();
                                Page::DiskComplete
                            };
                        }
                    }
                }
            }
            Message::CancelJob => {
                let pid_path = if self.page == Page::Installing {
                    INSTALL_PID
                } else {
                    DISK_PID
                };
                if let Ok(pid) = fs::read_to_string(pid_path) {
                    let _ = Command::new("kill").arg(pid.trim()).status();
                }
                self.status = i18n::tr("operation_cancelled");
                self.page = Page::JobError;
            }
            Message::EnsureFullscreen => {
                if !self.diskulator_mode
                    && let Some(id) = self.core.main_window_id()
                {
                    return set_mode(id, window::Mode::Fullscreen);
                }
            }
            Message::RebootTick => {
                if self.reboot_seconds > 1 {
                    self.reboot_seconds -= 1;
                } else {
                    self.reboot_seconds = 0;
                    self.reboot_now();
                }
            }
            Message::RebootNow => self.reboot_now(),
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        match self.page {
            Page::Language => self.language_page(),
            Page::Home => self.home(),
            Page::InstallerWelcome => self.installer_welcome(),
            Page::License => self.license(),
            Page::InstallerDisk => self.installer_disk(),
            Page::InstallerConfirm => self.installer_confirm(),
            Page::Installing => self.installing(),
            Page::InstallComplete => self.install_complete(),
            Page::JobError => self.job_error(),
            Page::Diskulator => self.diskulator(),
            Page::DiskConfirm => self.disk_confirm(),
            Page::DiskWorking => self.disk_working(),
            Page::DiskComplete => self.disk_complete(),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        if matches!(self.page, Page::Installing | Page::DiskWorking) {
            cosmic::iced::time::every(Duration::from_millis(300)).map(|_| Message::PollJob)
        } else if self.page == Page::InstallComplete && self.reboot_seconds > 0 {
            cosmic::iced::time::every(Duration::from_secs(1)).map(|_| Message::RebootTick)
        } else {
            Subscription::none()
        }
    }
}

struct DiskulatorApp(Utilities);

impl Application for DiskulatorApp {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "com.man.Diskulator";

    fn core(&self) -> &Core {
        &self.0.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.0.core
    }

    fn init(mut core: Core, _flags: ()) -> (Self, Task<Message>) {
        core.window.show_headerbar = true;
        core.window.show_close = true;
        core.window.show_maximize = true;
        core.window.show_minimize = true;
        core.set_header_title(i18n::tr("diskulator").into());
        let disks = scan_disks();
        let selected = disks.first().map(|disk| disk.path.clone());
        (
            Self(Utilities {
                core,
                page: Page::Diskulator,
                language: i18n::current_locale(),
                disks,
                selected,
                operation: None,
                progress: 0.0,
                status: String::new(),
                diskulator_mode: true,
                reboot_seconds: 59,
            }),
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        self.0.update(message)
    }

    fn view(&self) -> Element<'_, Message> {
        self.0.view()
    }

    fn subscription(&self) -> Subscription<Message> {
        self.0.subscription()
    }
}

impl Utilities {
    fn reboot_now(&mut self) {
        self.reboot_seconds = 0;
        self.status = i18n::tr("rebooting");
        let _ = Command::new("sync").status();
        if let Err(error) = Command::new("/sbin/reboot").spawn() {
            self.status = format!("Could not reboot automatically: {error}");
        }
    }
    fn window<'a>(&self, title: String, body: Element<'a, Message>) -> Element<'a, Message> {
        let card = widget::column::with_capacity(3)
            .spacing(16)
            .align_x(Alignment::Center)
            .push(widget::text::title1(title))
            .push(body)
            .push(widget::space::vertical().height(4));
        let card = widget::container(card)
            .class(theme::Container::Card)
            .padding(20)
            .max_width(560);
        widget::container(card)
            .class(theme::Container::WindowBackground)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }

    fn utility_button<'a>(
        &self,
        icon: &'a str,
        title: String,
        message: Message,
    ) -> Element<'a, Message> {
        let content = widget::column::with_capacity(2)
            .spacing(12)
            .align_x(Alignment::Center)
            .push(widget::icon::from_name(icon).size(72))
            .push(widget::text::heading(title));
        widget::button::custom(content)
            .padding([28, 38])
            .on_press(message)
            .into()
    }

    fn selected_disk(&self) -> Option<&Disk> {
        let selected = self.selected.as_ref()?;
        self.disks.iter().find(|disk| &disk.path == selected)
    }

    fn language_page(&self) -> Element<'_, Message> {
        let selected_index = i18n::find_language(&self.language);

        let selections: Vec<String> = i18n::LANGUAGES
            .iter()
            .map(|lang| format!("{} — {}", lang.name, lang.native))
            .collect();

        let dropdown = widget::dropdown(selections, selected_index, |index| {
            Message::LanguageSelected(i18n::LANGUAGES[index].locale.to_string())
        })
        .width(Length::Fill)
        .placeholder(i18n::tr("search"));

        let mut body = widget::column::with_capacity(6)
            .spacing(18)
            .align_x(Alignment::Center)
            .push(widget::icon::from_name("Prefs_Locale").size(80))
            .push(widget::text::body(i18n::tr("language_prompt")).size(18))
            .push(dropdown);

        // When a language is selected, show its flag icon and a Continue button
        // whose label includes the selected language's native name.
        if let Some(index) = selected_index {
            let lang = &i18n::LANGUAGES[index];
            body = body.push(widget::icon::from_name(lang.flag).size(48)).push(
                widget::button::suggested(format!("{} · {}", i18n::tr("continue"), lang.native))
                    .on_press(Message::LanguageConfirmed)
                    .width(Length::Fill),
            );
        } else {
            body = body.push(
                widget::button::suggested(i18n::tr("continue"))
                    .on_press(Message::LanguageConfirmed)
                    .width(Length::Fill),
            );
        }

        // Cancel button to return to the home screen.
        body = body.push(
            widget::button::standard(i18n::tr("cancel"))
                .on_press(Message::Home)
                .width(Length::Fill),
        );

        self.window(i18n::tr("language"), body.into())
    }

    fn home(&self) -> Element<'_, Message> {
        let actions = widget::row::with_capacity(3)
            .spacing(22)
            .align_y(Alignment::Center)
            .push(self.utility_button(
                "com.man.Install",
                i18n::tr("install_man"),
                Message::Installer,
            ))
            .push(self.utility_button(
                "com.system76.CosmicTerm",
                i18n::tr("terminal"),
                Message::Terminal,
            ))
            .push(self.utility_button(
                "com.man.Diskulator",
                i18n::tr("diskulator"),
                Message::Diskulator,
            ));
        self.window(i18n::tr("app_title"), actions.into())
    }

    fn installer_welcome(&self) -> Element<'_, Message> {
        let body = widget::column::with_capacity(5)
            .spacing(18)
            .align_x(Alignment::Center)
            .push(widget::icon::from_name("com.man.Install").size(96))
            .push(widget::text::heading(i18n::tr("set_up")))
            .push(widget::text::body(i18n::tr("set_up_desc")))
            .push(
                widget::row::with_capacity(2)
                    .spacing(12)
                    .push(widget::button::standard(i18n::tr("back")).on_press(Message::Home))
                    .push(
                        widget::button::suggested(i18n::tr("continue"))
                            .on_press(Message::InstallerLicense),
                    ),
            );
        self.window(i18n::tr("install_man"), body.max_width(680).into())
    }

    fn license(&self) -> Element<'_, Message> {
        let agreement = widget::container(
            widget::scrollable(widget::text::body(LICENSE)).height(Length::Fixed(260.0)),
        )
        .class(theme::Container::Secondary)
        .padding(20)
        .width(Length::Fill);
        let body = widget::column::with_capacity(4)
            .spacing(18)
            .push(widget::text::heading(i18n::tr("license_title")))
            .push(agreement)
            .push(
                widget::row::with_capacity(3)
                    .spacing(12)
                    .push(
                        widget::button::standard(i18n::tr("back"))
                            .on_press(Message::LicenseDisagree),
                    )
                    .push(widget::space::horizontal())
                    .push(
                        widget::button::suggested(i18n::tr("agree"))
                            .on_press(Message::LicenseAgree),
                    ),
            );
        self.window(
            i18n::tr("license_title"),
            body.width(Length::Fill).max_width(760).into(),
        )
    }

    fn installer_disk(&self) -> Element<'_, Message> {
        let mut list = widget::column::with_capacity(self.disks.len() + 4)
            .spacing(10)
            .push(widget::text::heading(i18n::tr("select_disk_subtitle")));
        let available: Vec<_> = self.disks.iter().filter(|disk| !disk.mounted).collect();
        if available.is_empty() {
            list = list.push(widget::text::body(i18n::tr("no_unmounted_disks")));
        }
        for disk in available {
            let label = format!(
                "{}    {}{}",
                disk.name,
                disk.size,
                if disk.removable {
                    format!("    {}", i18n::tr("removable"))
                } else {
                    "".into()
                }
            );
            let button = if self.selected.as_ref() == Some(&disk.path) {
                widget::button::suggested(label)
            } else {
                widget::button::standard(label)
            };
            list = list.push(
                button
                    .on_press(Message::SelectDisk(disk.path.clone()))
                    .width(Length::Fill),
            );
        }
        list = list.push(
            widget::row::with_capacity(3)
                .spacing(12)
                .push(
                    widget::button::standard(i18n::tr("back")).on_press(Message::InstallerLicense),
                )
                .push(widget::space::horizontal())
                .push(
                    widget::button::suggested(i18n::tr("continue"))
                        .on_press_maybe(self.selected.as_ref().map(|_| Message::InstallerConfirm)),
                ),
        );
        self.window(
            i18n::tr("choose_disk"),
            list.width(Length::Fill).max_width(720).into(),
        )
    }

    fn installer_confirm(&self) -> Element<'_, Message> {
        let destination = self
            .selected_disk()
            .map(|d| format!("{} ({}, {})", d.name, d.path, d.size))
            .unwrap_or_default();
        let body = widget::column::with_capacity(6)
            .spacing(18)
            .align_x(Alignment::Center)
            .push(widget::icon::from_name("dialog-warning-symbolic").size(72))
            .push(widget::text::heading(i18n::tr("erase_confirm_heading")))
            .push(widget::text::body(destination))
            .push(widget::text::body(i18n::tr("erase_destroy_all")))
            .push(
                widget::row::with_capacity(2)
                    .spacing(12)
                    .push(
                        widget::button::standard(i18n::tr("back")).on_press(Message::LicenseAgree),
                    )
                    .push(
                        widget::button::destructive(i18n::tr("erase_install"))
                            .on_press(Message::BeginInstall),
                    ),
            );
        self.window(i18n::tr("confirm_installation"), body.max_width(680).into())
    }

    // Only this active-install page follows the supplied visual reference.
    fn installing(&self) -> Element<'_, Message> {
        let default_disk = i18n::tr("selected_disk");
        let destination = self
            .selected_disk()
            .map(|d| d.name.as_str())
            .unwrap_or(&default_disk);
        let minutes = (((100.0 - self.progress) / 8.0).ceil() as u32).max(1);
        let body = widget::column::with_capacity(8)
            .spacing(16)
            .align_x(Alignment::Center)
            .push(
                widget::container(widget::icon::from_name("com.man.Install").size(112))
                    .class(theme::Container::Card)
                    .padding(14),
            )
            .push(widget::text::title1(i18n::tr("installing")))
            .push(widget::text::body(format!(
                "{} “{destination}”.",
                i18n::tr("installing_on_disk")
            )))
            .push(widget::space::vertical().height(16))
            .push(
                widget::progress_bar::linear::Linear::new()
                    .progress(self.progress / 100.0)
                    .width(Length::Fixed(520.0)),
            )
            .push(widget::text::body(&self.status))
            .push(widget::text::body(format!(
                "{}",
                i18n::tr("about_minutes").replace("{0}", &minutes.to_string())
            )))
            .push(widget::button::standard(i18n::tr("cancel")).on_press(Message::CancelJob));
        widget::container(body)
            .class(theme::Container::WindowBackground)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }

    fn install_complete(&self) -> Element<'_, Message> {
        let remaining = self.reboot_seconds as f32 / 59.0;
        let countdown = if self.reboot_seconds == 0 {
            self.status.clone()
        } else {
            i18n::tr("rebooting_in").replace("{0}", &self.reboot_seconds.to_string())
        };
        let body = widget::column::with_capacity(8)
            .spacing(18)
            .align_x(Alignment::Center)
            .push(widget::icon::from_name("emblem-ok-symbolic").size(88))
            .push(widget::text::heading(i18n::tr("install_complete")))
            .push(widget::text::body(i18n::tr("install_complete_desc")))
            .push(
                widget::progress_bar::linear::Linear::new()
                    .progress(remaining)
                    .width(Length::Fixed(520.0)),
            )
            .push(widget::text::body(countdown))
            .push(widget::button::suggested(i18n::tr("reboot_now")).on_press(Message::RebootNow));
        self.window(i18n::tr("install_complete"), body.max_width(680).into())
    }

    fn job_error(&self) -> Element<'_, Message> {
        let body = widget::column::with_capacity(4)
            .spacing(18)
            .align_x(Alignment::Center)
            .push(widget::icon::from_name("dialog-error-symbolic").size(72))
            .push(widget::text::heading(i18n::tr("operation_stopped")))
            .push(widget::text::body(&self.status))
            .push(
                widget::button::standard(i18n::tr("return_to_utilities")).on_press(Message::Home),
            );
        self.window(i18n::tr("operation_stopped"), body.max_width(680).into())
    }

    fn diskulator(&self) -> Element<'_, Message> {
        let mut list = widget::column::with_capacity(self.disks.len() + 3)
            .spacing(10)
            .push(widget::text::heading(i18n::tr("devices")))
            .push(widget::text::caption(i18n::tr("storage_sub")));
        for disk in &self.disks {
            let selected = self.selected.as_ref() == Some(&disk.path);
            let label = widget::column::with_capacity(2)
                .spacing(3)
                .push(widget::text::body(format!(
                    "{}{}",
                    if selected { "●  " } else { "" },
                    disk.name
                )))
                .push(widget::text::caption(format!(
                    "{}  ·  {}",
                    disk.path, disk.size
                )));
            list = list.push(
                widget::button::custom(label)
                    .class(if selected {
                        theme::Button::Suggested
                    } else {
                        theme::Button::Standard
                    })
                    .padding(10)
                    .on_press(Message::SelectDisk(disk.path.clone()))
                    .width(Length::Fill),
            );
        }
        if self.disks.is_empty() {
            list = list.push(widget::text::body(i18n::tr("no_disks")));
        }
        list = list
            .push(widget::button::standard(i18n::tr("refresh")).on_press(Message::BackToDiskList));
        let sidebar = widget::container(widget::scrollable(list).height(Length::Fill))
            .class(theme::Container::Secondary)
            .padding(18)
            .width(Length::Fixed(280.0))
            .height(Length::Fill);

        let main: Element<'_, Message> = if let Some(disk) = self.selected_disk() {
            let partitioned = disk
                .partitions
                .iter()
                .map(|partition| partition.bytes)
                .sum::<u64>()
                .min(disk.bytes);
            let unallocated = disk.bytes.saturating_sub(partitioned);
            let ratio = if disk.bytes == 0 {
                0.0
            } else {
                partitioned as f32 / disk.bytes as f32
            };
            let state = if disk.mounted {
                i18n::tr("in_use")
            } else {
                i18n::tr("ready")
            };
            let kind = if disk.removable {
                i18n::tr("removable_disk")
            } else {
                i18n::tr("internal_disk")
            };

            let header = widget::row::with_capacity(3)
                .spacing(18)
                .align_y(Alignment::Center)
                .push(widget::icon::from_name("com.man.Diskulator").size(72))
                .push(
                    widget::column::with_capacity(3)
                        .spacing(3)
                        .push(widget::text::title1(&disk.name))
                        .push(widget::text::body(&disk.path))
                        .push(widget::text::caption(format!("{kind} · {state}"))),
                )
                .push(widget::space::horizontal())
                .push(widget::text::title2(&disk.size));

            let destructive = !disk.mounted;
            let toolbar = widget::row::with_capacity(4)
                .spacing(10)
                .push(
                    widget::button::standard(i18n::tr("first_aid"))
                        .on_press(Message::ChooseDiskOperation(DiskOperation::FirstAid)),
                )
                .push(
                    widget::button::suggested(i18n::tr("partition_man")).on_press_maybe(
                        destructive
                            .then_some(Message::ChooseDiskOperation(DiskOperation::PartitionMan)),
                    ),
                )
                .push(
                    widget::button::destructive(i18n::tr("erase_fat32")).on_press_maybe(
                        destructive
                            .then_some(Message::ChooseDiskOperation(DiskOperation::EraseFat32)),
                    ),
                )
                .push(
                    widget::button::destructive(i18n::tr("erase_ext2")).on_press_maybe(
                        destructive
                            .then_some(Message::ChooseDiskOperation(DiskOperation::EraseExt2)),
                    ),
                );

            let capacity = widget::container(
                widget::column::with_capacity(5)
                    .spacing(12)
                    .push(widget::text::heading(i18n::tr("capacity_map")))
                    .push(
                        widget::progress_bar::linear::Linear::new()
                            .progress(ratio)
                            .width(Length::Fill),
                    )
                    .push(
                        widget::row::with_capacity(3)
                            .spacing(22)
                            .push(widget::text::body(format!(
                                "{}  {}",
                                i18n::tr("partitioned"),
                                human_size(partitioned)
                            )))
                            .push(widget::text::body(format!(
                                "{}  {}",
                                i18n::tr("unallocated"),
                                human_size(unallocated)
                            )))
                            .push(widget::text::body(format!(
                                "{}  {}",
                                i18n::tr("partition_count"),
                                disk.partitions.len()
                            ))),
                    ),
            )
            .class(theme::Container::Card)
            .padding(18)
            .width(Length::Fill);

            let mut partitions = widget::column::with_capacity(disk.partitions.len() + 2)
                .spacing(10)
                .push(widget::text::heading(i18n::tr("volumes_partitions")));
            if disk.partitions.is_empty() {
                partitions = partitions.push(
                    widget::container(
                        widget::column::with_capacity(2)
                            .spacing(5)
                            .push(widget::text::body(i18n::tr("no_partition_table")))
                            .push(widget::text::caption(i18n::tr("init_disk"))),
                    )
                    .class(theme::Container::Secondary)
                    .padding(16)
                    .width(Length::Fill),
                );
            }
            for partition in &disk.partitions {
                let summary = widget::row::with_capacity(3)
                    .align_y(Alignment::Center)
                    .push(
                        widget::column::with_capacity(2)
                            .spacing(3)
                            .push(widget::text::heading(&partition.path))
                            .push(widget::text::caption(
                                i18n::tr("filesystem_mount")
                                    .replace("{fs}", &partition.filesystem)
                                    .replace("{mount}", &partition.mountpoint),
                            )),
                    )
                    .push(widget::space::horizontal())
                    .push(widget::text::body(&partition.size));
                partitions = partitions.push(
                    widget::container(summary)
                        .class(theme::Container::Secondary)
                        .padding(14)
                        .width(Length::Fill),
                );
            }

            widget::scrollable(
                widget::column::with_capacity(6)
                    .spacing(20)
                    .push(header)
                    .push(toolbar)
                    .push(if disk.mounted {
                        widget::text::body(i18n::tr("mounted_warning"))
                    } else {
                        widget::text::body(i18n::tr("choose_operation"))
                    })
                    .push(capacity)
                    .push(partitions),
            )
            .height(Length::Fill)
            .into()
        } else {
            widget::container(
                widget::column::with_capacity(3)
                    .spacing(12)
                    .align_x(Alignment::Center)
                    .push(widget::icon::from_name("com.man.Diskulator").size(96))
                    .push(widget::text::heading(i18n::tr("select_disk")))
                    .push(widget::text::body(i18n::tr("select_disk_desc"))),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        };

        widget::container(
            widget::row::with_capacity(2)
                .push(sidebar)
                .push(widget::container(main).padding(28).width(Length::Fill)),
        )
        .class(theme::Container::WindowBackground)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn disk_confirm(&self) -> Element<'_, Message> {
        let operation = self.operation.unwrap_or(DiskOperation::FirstAid);
        let disk_label = self
            .selected_disk()
            .map(|d| format!("{} ({}, {})", d.name, d.path, d.size))
            .unwrap_or_default();
        let warning = if operation.destructive() {
            i18n::tr("erase_warning")
        } else {
            i18n::tr("first_aid_warning")
        };
        let title = operation.title();
        let action = if operation.destructive() {
            widget::button::destructive(title.clone()).on_press(Message::BeginDiskOperation)
        } else {
            widget::button::suggested(title.clone()).on_press(Message::BeginDiskOperation)
        };
        let body = widget::column::with_capacity(6)
            .spacing(18)
            .align_x(Alignment::Center)
            .push(widget::icon::from_name("com.man.Diskulator").size(82))
            .push(widget::text::heading(title))
            .push(widget::text::body(disk_label))
            .push(widget::text::body(operation.description()))
            .push(widget::text::body(warning))
            .push(
                widget::row::with_capacity(2)
                    .spacing(12)
                    .push(
                        widget::button::standard(i18n::tr("cancel"))
                            .on_press(Message::BackToDiskList),
                    )
                    .push(action),
            );
        self.window(i18n::tr("disk_confirm"), body.max_width(700).into())
    }

    fn disk_working(&self) -> Element<'_, Message> {
        let title = self
            .operation
            .map(DiskOperation::title)
            .unwrap_or_else(|| i18n::tr("working"));
        let body = widget::column::with_capacity(6)
            .spacing(18)
            .align_x(Alignment::Center)
            .push(widget::icon::from_name("com.man.Diskulator").size(90))
            .push(widget::text::heading(title))
            .push(
                widget::progress_bar::linear::Linear::new()
                    .progress(self.progress / 100.0)
                    .width(Length::Fixed(520.0)),
            )
            .push(widget::text::body(&self.status))
            .push(widget::button::standard(i18n::tr("cancel")).on_press(Message::CancelJob));
        self.window(i18n::tr("diskulator"), body.max_width(700).into())
    }

    fn disk_complete(&self) -> Element<'_, Message> {
        let body = widget::column::with_capacity(4)
            .spacing(18)
            .align_x(Alignment::Center)
            .push(widget::icon::from_name("emblem-ok-symbolic").size(82))
            .push(widget::text::heading(i18n::tr("disk_op_complete")))
            .push(widget::text::body(&self.status))
            .push(widget::button::suggested(i18n::tr("done")).on_press(Message::BackToDiskList));
        self.window(i18n::tr("diskulator"), body.max_width(680).into())
    }
}

fn start_backend(
    program: &str,
    args: &[&str],
    status: &str,
    pid: &str,
    log: &str,
) -> Result<(), String> {
    let _ = fs::remove_file(status);
    let _ = fs::remove_file(pid);
    let stdout = File::create(log).map_err(|error| error.to_string())?;
    let stderr = stdout.try_clone().map_err(|error| error.to_string())?;
    let child = Command::new(program)
        .args(args)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .map_err(|error| format!("Could not start {program}: {error}"))?;
    fs::write(pid, format!("{}\n", child.id())).map_err(|error| error.to_string())
}

fn read_status(path: &str) -> Option<(i32, String)> {
    let value = fs::read_to_string(path).ok()?;
    let (progress, message) = value.trim().split_once('|')?;
    Some((progress.parse().ok()?, message.to_string()))
}

fn scan_disks() -> Vec<Disk> {
    let mounts = fs::read_to_string("/proc/mounts").unwrap_or_default();
    let Ok(entries) = fs::read_dir("/sys/class/block") else {
        return Vec::new();
    };
    let mut disks = Vec::new();
    for entry in entries.flatten() {
        if entry.path().join("partition").exists() {
            continue;
        }
        let kernel_name = entry.file_name().to_string_lossy().into_owned();
        if kernel_name.starts_with("loop")
            || kernel_name.starts_with("sr")
            || kernel_name.starts_with("ram")
        {
            continue;
        }
        let sectors = read_number(entry.path().join("size"));
        if sectors == 0 {
            continue;
        }
        let path = format!("/dev/{kernel_name}");
        let model = fs::read_to_string(entry.path().join("device/model"))
            .unwrap_or_default()
            .trim()
            .to_string();
        let name = if model.is_empty() {
            kernel_name.clone()
        } else {
            model
        };
        let removable = read_number(entry.path().join("removable")) == 1;
        let mut partitions = Vec::new();
        if let Ok(children) = fs::read_dir(entry.path()) {
            for child in children.flatten() {
                if !child.path().join("partition").exists() {
                    continue;
                }
                let part_name = child.file_name().to_string_lossy().into_owned();
                let part_path = format!("/dev/{part_name}");
                let mountpoint = mounts
                    .lines()
                    .find_map(|line| {
                        let mut fields = line.split_whitespace();
                        (fields.next()? == part_path)
                            .then(|| fields.next().unwrap_or("").to_string())
                    })
                    .unwrap_or_else(|| "Not mounted".into());
                let filesystem = Command::new("blkid")
                    .args(["-o", "value", "-s", "TYPE", &part_path])
                    .output()
                    .ok()
                    .filter(|output| output.status.success())
                    .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| "Unknown".into());
                let partition_bytes = read_number(child.path().join("size")) * 512;
                partitions.push(Partition {
                    path: part_path,
                    bytes: partition_bytes,
                    size: human_size(partition_bytes),
                    filesystem,
                    mountpoint,
                });
            }
        }
        partitions.sort_by(|a, b| a.path.cmp(&b.path));
        let mounted = mounts.lines().any(|line| {
            line.split_whitespace()
                .next()
                .is_some_and(|source| source.starts_with(&path))
        });
        disks.push(Disk {
            path,
            name,
            size: human_size(sectors * 512),
            bytes: sectors * 512,
            removable,
            mounted,
            partitions,
        });
    }
    disks.sort_by(|a, b| a.path.cmp(&b.path));
    disks
}

fn read_number(path: impl AsRef<std::path::Path>) -> u64 {
    fs::read_to_string(path)
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(0)
}

fn human_size(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GiB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MiB", bytes as f64 / 1_048_576.0)
    } else {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    }
}
