use cosmic::app::{Command, Core};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length, Padding, Pixels};
use cosmic::iced_widget::combo_box::State;
use cosmic::widget::icon::Named;
use cosmic::widget::{self, icon, list_column, menu, nav_bar};
use cosmic::{cosmic_theme, theme, Application, ApplicationExt, Apply, Element};
use itertools::Itertools;
use quickemu::config::Arch;
use quickget_core::data_structures::Config;
use quickget_core::QuickgetInstance;
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
    SelectedRelease(String),
    SelectedEdition(String),
    SelectedArch(Arch),
    SetRAM(f64),
    SetCPUCores(usize),
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
    release_list: State<String>,
    release: Option<String>,
    edition_list: Option<State<String>>,
    edition: Option<String>,
    arch_list: State<Arch>,
    arch: Option<Arch>,
    cpu_cores: usize,
    ram: f64,
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
                let release_list = State::new(
                    os.releases
                        .clone()
                        .into_iter()
                        .filter_map(|config| config.release)
                        .unique()
                        .collect(),
                );
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
                let arch_list = State::new(arch_list);

                let ram =
                    QuickgetInstance::get_recommended_ram() as f64 / (1024 * 1024 * 1024) as f64;
                let cpu_cores = QuickgetInstance::get_recommended_cpu_cores();

                self.options = Some(OptionSelection {
                    config_list: os.releases,
                    release: None,
                    release_list,
                    edition: None,
                    edition_list: None,
                    arch,
                    arch_list,
                    ram,
                    cpu_cores,
                });
                self.page = Page::Options;
            }
            Message::SelectedRelease(input_release) => {
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
                    *release = Some(input_release);
                    if release.is_some() {
                        let editions = config_list
                            .clone()
                            .into_iter()
                            .filter(|config| &config.release == release)
                            .filter_map(|config| config.edition)
                            .collect::<Vec<_>>();
                        let editions = (!editions.is_empty()).then_some(editions);
                        if let Some(current_edition) = edition {
                            if !editions
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

                            *arch_list = State::new(
                                [Arch::x86_64, Arch::aarch64, Arch::riscv64]
                                    .into_iter()
                                    .filter(|a| full_arch_list.contains(a))
                                    .collect(),
                            );
                        }
                        *edition_list = editions.map(State::new);
                    } else {
                        *edition_list = None;
                        *edition = None;
                    }
                }
            }
            Message::SelectedEdition(input_edition) => {
                if let Some(OptionSelection {
                    config_list,
                    edition,
                    edition_list,
                    release,
                    arch_list,
                    ..
                }) = &mut self.options
                {
                    *edition = Some(input_edition);
                    let full_arch_list = config_list
                        .clone()
                        .into_iter()
                        .filter(|config| &config.release == release)
                        .filter(|config| &config.edition == edition)
                        .map(|config| config.arch)
                        .collect::<Vec<_>>();
                    *arch_list = State::new(
                        [Arch::x86_64, Arch::aarch64, Arch::riscv64]
                            .into_iter()
                            .filter(|a| full_arch_list.contains(a))
                            .collect(),
                    );
                }
            }
            Message::SelectedArch(input_arch) => {
                if let Some(OptionSelection {
                    config_list,
                    release,
                    edition,
                    edition_list,
                    release_list,
                    arch,
                    ..
                }) = &mut self.options
                {
                    *arch = Some(input_arch);
                    let releases = config_list
                        .clone()
                        .into_iter()
                        .filter(|config| Some(&config.arch) == arch.as_ref())
                        .filter_map(|config| config.release)
                        .unique()
                        .collect::<Vec<_>>();
                    if let Some(current_release) = release {
                        if !releases.contains(current_release) {
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
                    let editions = (!editions.is_empty()).then_some(editions);
                    if let Some(current_edition) = edition {
                        if !editions
                            .as_ref()
                            .map_or(false, |list| list.contains(current_edition))
                        {
                            *edition = None;
                        }
                    }
                    *release_list = State::new(releases);
                    *edition_list = editions.map(State::new);
                }
            }
            Message::SetRAM(input_ram) => {
                if let Some(OptionSelection { ram, .. }) = &mut self.options {
                    *ram = input_ram;
                }
            }
            Message::SetCPUCores(input_cores) => {
                if let Some(OptionSelection { cpu_cores, .. }) = &mut self.options {
                    *cpu_cores = input_cores;
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
                    release,
                    edition,
                    arch,
                    release_list,
                    edition_list,
                    arch_list,
                    ram,
                    cpu_cores,
                    ..
                } = self.options.as_ref().unwrap();

                let mut list = widget::list_column();
                let mut row = widget::row();
                let release_dropdown =
                    widget::combo_box(release_list, "Release", release.as_ref(), |release| {
                        Message::SelectedRelease(release).into()
                    });
                row = row.push(release_dropdown);

                if let Some(edition_list) = edition_list {
                    let edition_dropdown =
                        widget::combo_box(edition_list, "Edition", edition.as_ref(), |edition| {
                            Message::SelectedEdition(edition).into()
                        });
                    row = row.push(edition_dropdown);
                }

                let arch_dropdown =
                    widget::combo_box(arch_list, "Architecture", arch.as_ref(), |arch| {
                        Message::SelectedArch(arch).into()
                    });
                row = row.push(arch_dropdown);
                list = list.add(row);

                let total_cores = QuickgetInstance::get_total_cpu_cores() as f64;
                let cpu_slider = widget::slider(1.0..=total_cores, *cpu_cores as f64, |x| {
                    Message::SetCPUCores(x as usize).into()
                });
                let cpu_text = widget::text("CPU Cores:  ").width(Length::Shrink);
                let selected_cpu_text =
                    widget::text(format!("  {cpu_cores}")).width(Length::Shrink);
                let cpu_row = widget::row()
                    .push(cpu_text)
                    .push(cpu_slider)
                    .push(selected_cpu_text);
                list = list.add(cpu_row);

                let ram_gb = QuickgetInstance::get_total_ram() as f64 / (1024 * 1024 * 1024) as f64;
                let ram_slider =
                    widget::slider(0.25..=ram_gb as f64, *ram, |x| Message::SetRAM(x).into())
                        .step(0.01);
                let ram_text = widget::text("RAM:  ").width(Length::Shrink);
                let selected_ram_text =
                    widget::text(format!("  {ram:.2} GiB")).width(Length::Shrink);
                let ram_row = widget::row()
                    .push(ram_text)
                    .push(ram_slider)
                    .push(selected_ram_text);
                list = list.add(ram_row);

                list.into()
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
