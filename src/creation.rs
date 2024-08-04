use cosmic::app::{Command, Core};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length, Padding, Pixels};
use cosmic::widget::icon::Named;
use cosmic::widget::{self, icon, menu, nav_bar};
use cosmic::{cosmic_theme, theme, Application, ApplicationExt, Apply, Element};
use itertools::Itertools;
use quickemu::config::Arch;
use quickget_core::data_structures::Config;
use quickget_core::{data_structures::OS, ConfigSearch, ConfigSearchError, QGDownload};

#[derive(Default, Clone, Debug)]
pub struct Creation {
    os_list: Vec<OS>,
    page: Page,
    options: Option<OptionSelection>,
}

#[derive(Clone, Debug)]
pub enum Message {
    OSList(Result<Vec<OS>, String>),
    SelectedOS(OS),
    SelectedRelease(usize),
    SelectedEdition(usize),
    SelectedArch(usize),
}

#[derive(Clone, Debug, Default)]
enum Page {
    #[default]
    Loading,
    SelectOS,
    Options,
    Downloading(Vec<QGDownload>),
    Docker,
    Complete,
    Error(String),
}

#[derive(Clone, Debug)]
struct OptionSelection {
    config_list: Vec<Config>,
    release_list: Vec<String>,
    release: Option<String>,
    edition_list: Option<Vec<String>>,
    edition: Option<String>,
    arch_list: Vec<Arch>,
    arch: Option<Arch>,
}

impl Creation {
    pub fn new() -> Self {
        Self {
            os_list: vec![],
            page: Page::Loading,
            ..Default::default()
        }
    }
    pub fn update(&mut self, message: Message) {
        match message {
            Message::OSList(list) => match list {
                Ok(os_list) => {
                    self.os_list = os_list;
                    self.page = Page::SelectOS;
                }
                Err(e) => {
                    self.page = Page::Error(e);
                }
            },
            Message::SelectedOS(os) => {
                let release_list = os
                    .releases
                    .clone()
                    .into_iter()
                    .filter_map(|config| config.release)
                    .unique()
                    .collect::<Vec<String>>();
                let arch_list = [Arch::x86_64, Arch::aarch64, Arch::riscv64]
                    .into_iter()
                    .filter(|arch| os.releases.iter().any(|config| &config.arch == arch))
                    .collect::<Vec<Arch>>();

                let preferred_arch = match std::env::consts::ARCH {
                    "aarch64" => Arch::aarch64,
                    "riscv64" => Arch::riscv64,
                    _ => Arch::x86_64,
                };
                let arch = (arch_list.contains(&preferred_arch)).then_some(preferred_arch);

                self.options = Some(OptionSelection {
                    config_list: os.releases,
                    release: None,
                    release_list,
                    edition: None,
                    edition_list: None,
                    arch,
                    arch_list,
                });
                self.page = Page::Options;
            }
            Message::SelectedRelease(index) => {
                if let Some(OptionSelection {
                    config_list,
                    release,
                    release_list,
                    edition_list,
                    edition,
                    arch_list,
                    ..
                }) = &mut self.options
                {
                    *release = release_list.get(index).cloned();
                    if release.is_some() {
                        let editions = config_list
                            .clone()
                            .into_iter()
                            .filter(|config| &config.release == release)
                            .filter_map(|config| config.edition)
                            .collect::<Vec<_>>();
                        if editions.is_empty() {
                            *edition_list = None;
                        } else {
                            *edition_list = Some(editions);
                        }
                    } else {
                        *edition_list = None;
                    }
                    if let Some(current_edition) = edition {
                        if !edition_list
                            .as_ref()
                            .map_or(false, |list| list.contains(current_edition))
                        {
                            *edition = None;
                        }
                        let full_arch_list = config_list
                            .clone()
                            .into_iter()
                            .filter(|config| &config.release == release)
                            .filter(|config| &config.edition == edition)
                            .map(|config| config.arch)
                            .collect::<Vec<_>>();
                        *arch_list = [Arch::x86_64, Arch::aarch64, Arch::riscv64]
                            .into_iter()
                            .filter(|a| full_arch_list.contains(a))
                            .collect();
                    }
                }
            }
            Message::SelectedEdition(index) => {
                if let Some(OptionSelection {
                    config_list,
                    edition,
                    edition_list,
                    release,
                    arch_list,
                    ..
                }) = &mut self.options
                {
                    *edition = edition_list.as_ref().unwrap().get(index).cloned();
                    let full_arch_list = config_list
                        .clone()
                        .into_iter()
                        .filter(|config| &config.release == release)
                        .filter(|config| &config.edition == edition)
                        .map(|config| config.arch)
                        .collect::<Vec<_>>();
                    *arch_list = [Arch::x86_64, Arch::aarch64, Arch::riscv64]
                        .into_iter()
                        .filter(|a| full_arch_list.contains(a))
                        .collect();
                }
            }
            Message::SelectedArch(index) => {
                if let Some(OptionSelection {
                    config_list,
                    release,
                    edition,
                    edition_list,
                    release_list,
                    arch,
                    arch_list,
                }) = &mut self.options
                {
                    *arch = arch_list.get(index).cloned();
                    *release_list = config_list
                        .clone()
                        .into_iter()
                        .filter(|config| Some(&config.arch) == arch.as_ref())
                        .filter_map(|config| config.release)
                        .unique()
                        .collect();
                    if let Some(current_release) = release {
                        if !release_list.contains(current_release) {
                            *release = None;
                        }
                    }

                    let editions = config_list
                        .clone()
                        .into_iter()
                        .filter(|config| Some(&config.arch) == arch.as_ref())
                        .filter(|config| &config.release == release)
                        .filter_map(|config| config.edition)
                        .collect::<Vec<_>>();
                    if editions.is_empty() {
                        *edition_list = None;
                    } else {
                        *edition_list = Some(editions);
                    }
                    if let Some(current_edition) = edition {
                        if !edition_list
                            .as_ref()
                            .map_or(false, |list| list.contains(current_edition))
                        {
                            *edition = None;
                        }
                    }
                }
            }
        }
    }
    pub fn view(&self) -> Element<crate::app::Message> {
        match self.page {
            Page::Loading => widget::text("loading")
                .apply(widget::container)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .into(),
            Page::SelectOS => {
                let mut list_column = widget::list_column().style(theme::Container::ContextDrawer);
                let os_list = self.os_list.clone();
                for os in os_list {
                    let mut row = widget::row().align_items(Alignment::End);
                    if let Some(homepage) = os.homepage.clone() {
                        let homepage_button =
                            widget::button::icon(icon::from_name("go-home-symbolic"))
                                .on_press(crate::app::Message::LaunchUrl(homepage))
                                .tooltip(format!("Visit {} homepage", os.pretty_name))
                                .width(Length::Shrink);
                        row = row.push(homepage_button);
                    }
                    let button = widget::button::text(os.pretty_name.clone())
                        .on_press(Message::SelectedOS(os).into())
                        .width(Length::Fill);
                    row = row.push(button);

                    list_column = list_column.add(row);
                }
                widget::scrollable(list_column).into()
            }
            Page::Options => {
                let OptionSelection {
                    release_list,
                    release,
                    config_list,
                    edition_list,
                    edition,
                    arch_list,
                    arch,
                    ..
                } = self.options.as_ref().unwrap();
                let mut list = widget::list_column();
                let release_position = release
                    .as_ref()
                    .map(|release| release_list.iter().position(|r| r == release).unwrap());
                let release_dropdown = widget::dropdown(release_list, release_position, |x| {
                    Message::SelectedRelease(x).into()
                });
                let release_row = widget::row()
                    .push(widget::text("Release: "))
                    .push(release_dropdown);
                list = list.add(release_row);

                if let Some(edition_list) = edition_list {
                    let edition_position = edition
                        .as_ref()
                        .map(|edition| edition_list.iter().position(|e| e == edition).unwrap());
                    let edition_dropdown = widget::dropdown(edition_list, edition_position, |x| {
                        Message::SelectedEdition(x).into()
                    });
                    let edition_row = widget::row()
                        .push(widget::text("Edition: "))
                        .push(edition_dropdown);
                    list = list.add(edition_row);
                }

                let arch_position = arch
                    .as_ref()
                    .map(|arch| arch_list.iter().position(|a| a == arch).unwrap());
                let arch_dropdown = widget::dropdown(arch_list, arch_position, |x| {
                    Message::SelectedArch(x).into()
                });
                let arch_row = widget::row()
                    .push(widget::text("Arch: "))
                    .push(arch_dropdown);
                list = list.add(arch_row);

                widget::scrollable(list).into()
            }
            _ => widget::text("NOT YET IMPLEMENTED").into(),
        }
    }
}

impl From<Message> for crate::app::Message {
    fn from(val: Message) -> Self {
        crate::app::Message::Creation(val)
    }
}
