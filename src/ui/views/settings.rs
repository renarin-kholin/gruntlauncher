use std::path::PathBuf;

use iced::{
    Element, Length, Task,
    alignment::{Horizontal, Vertical},
    padding,
    widget::{
        bottom, button, center, column, image, right, row, rule, scrollable, space, text,
        text_input,
    },
};
use tracing::error;

use crate::{
    assets::GRUNT_BANNER,
    core::config::Config,
    services::config::{ConfigError, pick_folder, reset_config, save_config},
    ui::{GruntAction, GruntState, views::ScreenOutput},
};
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Tab {
    #[default]
    General,
    About,
}
pub struct Screen {
    selected_tab: Tab,
    instances_path: PathBuf,
    installations_path: PathBuf,
    changed: bool,
    error: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Message {
    Navigate(Tab),
    EditInstancesPath,
    EditInstallationsPath,

    InstancesPathChanged(String),
    InstallationsPathChanged(String),

    InstancePathPicked(Result<PathBuf, ConfigError>),
    InstallationsPathPicked(Result<PathBuf, ConfigError>),

    SaveConfig,
    DiscardConfig,
    ResetToDefault,
    ConfigSaved(Result<(), ConfigError>),
    ConfigReset(Result<Config, ConfigError>),
}

impl Screen {
    pub fn new(state: &mut GruntState) -> (Self, Task<Message>) {
        (
            Self {
                selected_tab: Tab::default(),
                instances_path: state.config.instances_folder.clone(),
                installations_path: state.config.installations_folder.clone(),
                changed: false,
                error: None,
            },
            Task::none(),
        )
    }
    pub fn changed_maybe(&self, m: Message) -> Option<Message> {
        if self.changed { Some(m) } else { None }
    }
    pub fn view_general<'a>(&'a self, _state: &'a GruntState) -> Element<'a, Message> {
        use Message::*;
        let mut form = column![
            text!("Instances Path"),
            row![
                text_input(
                    "Path to the instances",
                    &format!("{}", self.instances_path.display())
                )
                .on_input(InstancesPathChanged),
                button("Edit").on_press(EditInstancesPath)
            ]
            .spacing(10.0)
            .align_y(Vertical::Center),
            space().height(10.0),
            row![
                text_input(
                    "Path to the installations",
                    &format!("{}", self.installations_path.display())
                )
                .on_input(InstallationsPathChanged),
                button("Edit").on_press(EditInstallationsPath)
            ]
            .spacing(10.0)
            .align_y(Vertical::Center),
        ]
        .spacing(5.0)
        .padding(10.0);

        if let Some(e) = &self.error {
            form = form.push(text!("Error: {e}"));
        }
        form = form.push(bottom(right(
            row![
                button("Reset to defaults").on_press(ResetToDefault),
                button("Discard").on_press_maybe(self.changed_maybe(DiscardConfig)),
                button("Save").on_press_maybe(self.changed_maybe(SaveConfig))
            ]
            .spacing(10.0),
        )));
        form.into()
    }
    pub fn view_about<'a>(&'a self, _state: &'a GruntState) -> Element<'a, Message> {
        let icon = image::Handle::from_bytes(GRUNT_BANNER);
        center(
            column![
                image(icon).height(200.0),
                row![space().width(Length::FillPortion(1)),
                text!("GruntLauncher manages multiple Vintage Story game versions and instances side by side, so you can keep separate mod setups for different worlds or versions without them stepping on each other which is similar in spirit to launchers like Prism/MultiMC for Minecraft.").width(Length::FillPortion(2)).center(), 
space().width(Length::FillPortion(1))
                ],
                text!("Version {}", env!("CARGO_PKG_VERSION"))
            ]
            .align_x(Horizontal::Center)
            .padding(10.0)
            .spacing(10.0)).into()
    }
    pub fn view<'a>(&'a self, state: &'a GruntState) -> Element<'a, Message> {
        use Message::*;
        row![
            scrollable(
                column![
                    button("General")
                        .on_press(Navigate(Tab::General))
                        .width(Length::Fill)
                        .style(|theme, mut status| {
                            if self.selected_tab == Tab::General {
                                status = button::Status::Pressed
                            };
                            button::subtle(theme, status)
                        }),
                    button("About")
                        .on_press(Navigate(Tab::About))
                        .width(Length::Fill)
                        .style(|theme, mut status| {
                            if self.selected_tab == Tab::About {
                                status = button::Status::Pressed
                            };
                            button::subtle(theme, status)
                        }),
                ]
                .padding(padding::all(10.0))
                .spacing(10.0)
            )
            .width(Length::Fixed(150.0)),
            rule::vertical(1.0),
            match self.selected_tab {
                Tab::General => self.view_general(state),
                Tab::About => self.view_about(state),
            }
        ]
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    }
    pub fn update(&mut self, message: Message, state: &mut GruntState) -> ScreenOutput<Message> {
        use Message::*;
        match message {
            Navigate(tab) => {
                self.selected_tab = tab;
            }
            EditInstancesPath => {
                return ScreenOutput::task(Task::perform(
                    pick_folder(state.config.instances_folder.clone()),
                    InstancePathPicked,
                ));
            }
            EditInstallationsPath => {
                return ScreenOutput::task(Task::perform(
                    pick_folder(state.config.installations_folder.clone()),
                    InstallationsPathPicked,
                ));
            }
            InstancePathPicked(result) => match result {
                Ok(path) => {
                    self.instances_path = path;
                    self.changed(state);
                }
                Err(e) => {
                    error!("{e}");
                }
            },
            InstallationsPathPicked(result) => match result {
                Ok(path) => {
                    self.installations_path = path;
                    self.changed(state);
                }
                Err(e) => {
                    error!("{e}");
                }
            },
            InstancesPathChanged(new_path) => {
                self.instances_path = PathBuf::from(new_path);
                self.changed(state);
            }
            InstallationsPathChanged(new_path) => {
                self.installations_path = PathBuf::from(new_path);
                self.changed(state);
            }
            SaveConfig => {
                state.config.instances_folder = self.instances_path.clone();
                state.config.installations_folder = self.installations_path.clone();
                return ScreenOutput::task(Task::perform(
                    save_config(state.config.clone()),
                    ConfigSaved,
                ));
            }
            ResetToDefault => {
                return ScreenOutput::task(Task::perform(reset_config(), ConfigReset));
            }
            DiscardConfig => {
                self.discard_changes(state);
            }
            ConfigSaved(result) => match result {
                Ok(()) => {
                    self.changed = false;
                    return ScreenOutput::action(GruntAction::ReloadInstances);
                }
                Err(e) => {
                    error!("{e}");
                    self.discard_changes(state);
                }
            },
            ConfigReset(result) => match result {
                Ok(config) => {
                    self.changed = false;
                    state.config = config;
                    return ScreenOutput::action(GruntAction::ReloadInstances);
                }
                Err(e) => {
                    error!("{e}");
                    self.discard_changes(state);
                }
            },
        }
        ScreenOutput::none()
    }
    fn discard_changes(&mut self, state: &mut GruntState) {
        self.instances_path = state.config.instances_folder.clone();
        self.installations_path = state.config.installations_folder.clone();
        self.changed = false;
    }
    fn changed(&mut self, state: &GruntState) {
        self.changed = self.instances_path != state.config.instances_folder
            || self.installations_path != state.config.installations_folder;
    }
}
