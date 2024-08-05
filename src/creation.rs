use std::path::PathBuf;

use ashpd::desktop::file_chooser::{FileFilter, SelectedFiles};
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
    None,
    OSList(Result<Vec<OS>, String>),
    SelectedOS(OS),
    SelectedRelease(String),
    SelectedEdition(String),
    SelectedArch(Arch),
    SetRAM(f64),
    SetCPUCores(usize),
    SelectVMDir,
    SelectedDir(PathBuf),
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
    directory: PathBuf,
}

impl OptionSelection {
    fn refresh(&mut self) {
        let releases = self
            .config_list
            .clone()
            .into_iter()
            .filter(|config| self.arch.as_ref().map_or(true, |arch| &config.arch == arch))
            .filter(|config| self.edition.is_none() || &config.edition == &self.edition)
            .filter_map(|config| config.release)
            .unique()
            .collect::<Vec<String>>();

        if let Some(ref release) = self.release {
            if !releases.contains(release) {
                self.release = None;
            }
        }
        self.release_list = State::new(releases);

        let editions = self.release.as_ref().and({
            let editions = self
                .config_list
                .clone()
                .into_iter()
                .filter(|config| self.arch.as_ref().map_or(true, |arch| &config.arch == arch))
                .filter(|config| self.release == config.release)
                .filter_map(|config| config.edition)
                .unique()
                .collect::<Vec<String>>();
            (!editions.is_empty()).then_some(editions)
        });
        if let Some(ref edition) = self.edition {
            if let Some(ref editions) = editions {
                if !editions.contains(edition) {
                    self.edition = None;
                }
            } else {
                self.edition = None;
            }
        }
        self.edition_list = editions.map(State::new);

        let full_arch_list = self
            .config_list
            .clone()
            .into_iter()
            .filter(|config| self.release.is_none() || config.release == self.release)
            .filter(|config| self.edition.is_none() || config.edition == self.edition)
            .map(|config| config.arch)
            .collect::<Vec<Arch>>();
        let arch_list = [Arch::x86_64, Arch::aarch64, Arch::riscv64]
            .into_iter()
            .filter(|arch| full_arch_list.contains(arch))
            .collect::<Vec<Arch>>();
        if let Some(ref arch) = self.arch {
            if !arch_list.contains(arch) {
                self.arch = None;
            }
        }
        self.arch_list = State::new(arch_list);
    }
    fn set_release(&mut self, release: String) {
        self.release = Some(release);
        self.refresh();
    }
    fn set_edition(&mut self, edition: String) {
        self.edition = Some(edition);
        self.refresh();
    }
    fn set_arch(&mut self, arch: Arch) {
        self.arch = Some(arch);
        self.refresh();
    }
}

impl Creation {
    pub fn new() -> Self {
        Self {
            os_list: vec![],
            page: Page::Loading,
            ..Default::default()
        }
    }
    pub fn update(&mut self, message: Message) -> Command<crate::app::Message> {
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
                    directory: std::env::current_dir().unwrap(),
                });
                self.page = Page::Options;
            }
            Message::SelectedRelease(release) => {
                if let Some(options) = &mut self.options {
                    options.set_release(release);
                }
            }
            Message::SelectedEdition(edition) => {
                if let Some(options) = &mut self.options {
                    options.set_edition(edition);
                }
            }
            Message::SelectedArch(arch) => {
                if let Some(options) = &mut self.options {
                    options.set_arch(arch);
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
            Message::SelectVMDir => {
                return Command::perform(
                    async move {
                        let result = SelectedFiles::open_file()
                            .title("Select VM Directory")
                            .accept_label("Select")
                            .modal(true)
                            .multiple(false)
                            .directory(true)
                            .send()
                            .await
                            .unwrap()
                            .response();

                        result.ok().and_then(|directory| {
                            directory
                                .uris()
                                .iter()
                                .next()
                                .and_then(|file| file.to_file_path().ok())
                        })
                    },
                    |directory| {
                        if let Some(directory) = directory {
                            crate::app::Message::Creation(Message::SelectedDir(directory)).into()
                        } else {
                            crate::app::Message::Creation(Message::None).into()
                        }
                    },
                );
            }
            Message::SelectedDir(selected_directory) => {
                if let Some(OptionSelection { directory, .. }) = &mut self.options {
                    *directory = selected_directory;
                    println!(
                        "Directory updated: {}. Exists: {}",
                        directory.display(),
                        directory.exists()
                    );
                }
            }
            Message::None => {}
        };
        Command::none()
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
                    directory,
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

                let vm_dir_text = widget::text("VM Directory:  ").width(Length::Shrink);
                let vm_dir_input = widget::text_input("VM Directory", directory.to_string_lossy())
                    .on_input(|dir| Message::SelectedDir(PathBuf::from(dir)).into());
                let vm_dir_open_button =
                    widget::button::icon(icon::from_name("folder-open-symbolic"))
                        .on_press(Message::SelectVMDir.into())
                        .tooltip("Select VM Directory")
                        .width(Length::Shrink);
                let vm_dir_row = widget::row()
                    .push(vm_dir_text)
                    .push(vm_dir_input)
                    .push(vm_dir_open_button);
                list = list.add(vm_dir_row);

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
