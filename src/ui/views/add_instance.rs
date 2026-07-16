use futures::stream::{self, StreamExt, TryStreamExt};
use std::{collections::HashMap, path::PathBuf};

use iced::{
    Element,
    Length::{self, Fill, Shrink},
    Task,
    alignment::{Horizontal, Vertical},
    padding,
    widget::{
        button, center, column, container, image, progress_bar, right, right_center, row, rule,
        scrollable, text, text_input,
    },
};
use iced_aw::spinner;
use thiserror::Error;
use tracing::error;

use crate::{
    assets::GRUNT_ICON,
    core::{
        game_mod::GameMod,
        instance::GruntInstance,
        version::{GameVersion, GameVersionSource, VersionCatalog},
    },
    services::{
        game_mod::{ModDetail, ModDownloadProgress, ModsError, Release, download_mod},
        image::{ImagesError, save_image},
        instance::{self, InstancesError},
        version::{
            InstallStatus, VersionsError, download_version, install_game, load_versions,
            refresh_versions,
        },
    },
    ui::{
        GruntAction, GruntState,
        component::mod_browser::{self, ModBrowser},
        views::ScreenOutput,
        widget::{
            overlay::overlay_container,
            table::{self, TableColumn},
        },
    },
};
use sipper::Sipper;
use sipper::{Straw, sipper};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    Basic,
    Mod,
    Review,
}
impl Step {
    pub fn next(&mut self) {
        match self {
            Self::Basic => *self = Self::Mod,
            Self::Mod => *self = Self::Review,
            Self::Review => *self = Self::Basic,
        }
    }
    pub fn back(&mut self) {
        match self {
            Self::Basic => *self = Self::Review,
            Self::Mod => *self = Self::Basic,
            Self::Review => *self = Self::Mod,
        }
    }
}
pub struct Screen {
    id: uuid::Uuid,
    name: String,
    selected_version: Option<GameVersion>,
    columns: Vec<TableColumn>,
    rows: Vec<Vec<String>>,
    icon_handle: image::Handle,
    step: Step,
    install_status: InstallStatus,
    mod_download_progress: HashMap<i64, ModDownloadProgress>,

    mod_browser: ModBrowser,
}

#[derive(Debug, Clone)]
pub enum Message {
    //Navigation events
    Navigate(Step),
    Next,
    Back,
    Cancel,

    //Basic Step events
    NameChanged(String),
    SelectVersion(usize),
    RefreshVersions,

    //Mod Step events
    ModBrowserMessage(mod_browser::Message),

    //Review Step events
    CreateInstance,
    RemoveMod(i64),

    //Webview events
    // ModViewPageFetched(Result<String, String>),

    //Service Result events
    VersionsLoaded(Result<Vec<GameVersion>, VersionsError>),
    InstanceInstalling(InstallStatus),
    InstanceInstalled(Result<(PathBuf, Vec<GameMod>), InstallError>),
    InstanceCreated(Result<GruntInstance, InstancesError>),
}

impl Screen {
    pub fn new(state: &mut GruntState) -> (Self, Task<Message>) {
        let mut screen = Self {
            id: uuid::Uuid::new_v4(),
            icon_handle: image::Handle::from_bytes(GRUNT_ICON),
            name: String::new(),
            selected_version: None,
            columns: vec![
                TableColumn::new("Version", 150.0).min_width(80.0),
                TableColumn::new("Type", 300.0).min_width(80.0),
            ],
            rows: vec![],
            step: Step::Basic,
            install_status: InstallStatus::NotStarted,
            mod_download_progress: HashMap::new(),

            mod_browser: ModBrowser::new(),
        };

        // Reuse an already-loaded catalog; otherwise kick off a load.
        let task = match &state.vs_versions {
            VersionCatalog::Loaded { versions } => {
                screen.rows = Self::version_rows(versions);
                Task::none()
            }
            _ => {
                state.vs_versions.loading();
                Task::perform(
                    load_versions(state.config.installations_folder.clone()),
                    Message::VersionsLoaded,
                )
            }
        };

        (screen, task)
    }

    fn version_rows(versions: &[GameVersion]) -> Vec<Vec<String>> {
        versions
            .iter()
            .map(|v| vec![v.version.to_string(), "Release".to_string()])
            .collect()
    }

    fn if_valid<Message>(&self, message: Message) -> Option<Message> {
        if !self.name.is_empty() && self.selected_version.is_some() {
            Some(message)
        } else {
            None
        }
    }
    fn view_basic<'a>(&'a self, state: &'a GruntState) -> Element<'a, Message> {
        use Message::*;
        let mut page_content = column![];
        page_content = if matches!(state.vs_versions, VersionCatalog::Loading) {
            page_content.push(
                center(
                    column![
                        text!("Loading Versions"),
                        spinner::Spinner::default().width(30.0).height(30.0)
                    ]
                    .align_x(Horizontal::Center)
                    .height(Shrink)
                    .spacing(10.0),
                )
                .height(Fill)
                .width(Fill),
            )
        } else {
            page_content.push(
                container(
                    table::Table::new(
                        &self.columns,
                        &self.rows,
                        self.selected_version.clone().map(|v| {
                            self.rows
                                .iter()
                                .position(|r| r[0] == v.version.to_string())
                                .unwrap_or(0)
                        }),
                    )
                    .row_height(30.0)
                    .on_select(SelectVersion),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(padding::all(1.0))
                .style(container::bordered_box),
            )
        };
        column![
            //Instance details (name and icon)
            row![
                button(image(self.icon_handle.clone()).height(50.0).width(50.0))
                    .style(button::text),
                column![
                    text!("Instance Name "),
                    text_input("Default name", &self.name).on_input(NameChanged)
                ]
                .spacing(5.0)
            ]
            .align_y(Vertical::Center)
            .spacing(10.0)
            .padding(padding::all(10.0))
            .height(Length::Shrink),
            rule::horizontal(1.0),
            button("Refresh Versions")
                .on_press(RefreshVersions)
                .style(move |theme, mut status| {
                    if matches!(state.vs_versions, VersionCatalog::Loading) {
                        status = button::Status::Disabled;
                    }
                    button::primary(theme, status)
                }),
            page_content,
            right(
                row![
                    button("Next")
                        .on_press_maybe(self.if_valid(Next))
                        .style(button::success),
                    button("Cancel").on_press(Cancel).style(button::danger)
                ]
                .spacing(10.0)
                .align_y(Vertical::Center)
            )
        ]
        .spacing(10.0)
        .padding(padding::all(10.0))
        .into()
    }
    fn review_mod_item(
        &self,
        mod_detail: Box<ModDetail>,
        release: &Release,
        state: &GruntState,
    ) -> Element<'_, Message> {
        use Message::*;
        let mut mod_logo = self.icon_handle.clone();
        if let Some(logo) = state.image_cache.peek(&mod_detail.modid) {
            mod_logo = logo.clone();
        }
        column![
            button(
                row![
                    container(image(mod_logo).height(50.0).width(50.0))
                        .style(container::bordered_box),
                    column![
                        text!("{}", mod_detail.name),
                        text!("{}", release.modversion)
                    ]
                    .spacing(5.0),
                    right_center(
                        row![button("Remove").on_press(RemoveMod(mod_detail.modid))]
                            .spacing(5.0)
                            .align_y(Vertical::Center)
                    )
                ]
                .padding(10.0)
                .spacing(10.0),
            )
            .style(button::subtle)
            .width(Length::Fill),
            rule::horizontal(1.0)
        ]
        .into()
    }
    fn view_progress_overlay(&self) -> Element<'_, Message> {
        use InstallStatus::*;

        let main_container = column![].spacing(10.0).padding(10.0);
        match &self.install_status {
            NotStarted => main_container.push(text!("Starting download please wait...")),
            Downloading { downloaded, total } => main_container
                .push(text!("Downloading game file"))
                .push(progress_bar(0.0..=(*total as f32), *downloaded as f32)),
            Verifying => main_container.push(text!("Verifying the download hash...")),
            Installing => main_container
                .push(text!("Installing"))
                .push(progress_bar(0.0..=100.0, 25.0)),
            DownloadingMods(..) => {
                main_container
                    .push(text!("Downloading Mods..."))
                    .push(scrollable(column(self.mod_download_progress.iter().map(
                        |(modid, mp)| {
                            use ModDownloadProgress::*;
                            let mut mod_progress = column![];
                            let selected_mod = self
                                .mod_browser
                                .selected_mods
                                .iter()
                                .find(|m| m.0.modid == *modid);
                            let mod_name = if let Some(selected_mod) = selected_mod {
                                selected_mod.0.name.clone()
                            } else {
                                "Unknown Mod".to_string()
                            };
                            mod_progress = match mp {
                                Queued => {
                                    mod_progress.push(text!("{} queued for download.", mod_name))
                                }
                                Downloading { downloaded, total } => mod_progress.push(column![
                                    text!("Downloading {}", mod_name),
                                    progress_bar(0.0..=(*total as f32), *downloaded as f32)
                                ]),
                                Downloaded => mod_progress.push(text!("Downloaded {}", mod_name)),
                                Failed(err) => mod_progress.push(text!(
                                    "Failed to download {}: {:?}",
                                    mod_name,
                                    err
                                )),
                            };
                            mod_progress.into()
                        },
                    ))))
            }
            Done => main_container.push(text!("Finished installing.")),
            Failed(e) => main_container.push(text!("{}", e)),
        }
        .into()
    }
    fn view_review(&self, state: &GruntState) -> Element<'_, Message> {
        use Message::*;

        let base = column![
            //Instance details (name and icon)
            row![
                button(image(self.icon_handle.clone()).height(50.0).width(50.0))
                    .style(button::text),
                column![
                    text!("Instance Name "),
                    text!("{}", &self.name),
                    text!(
                        "{}",
                        self.selected_version
                            .as_ref()
                            .map(|v| v.version.to_string())
                            .unwrap_or_else(|| "No version selected".to_string())
                    )
                ]
                .spacing(5.0)
            ]
            .align_y(Vertical::Center)
            .spacing(10.0)
            .padding(padding::all(10.0))
            .height(Length::Shrink),
            rule::horizontal(1.0),
            container(text!("The following mods will be installed"))
                .padding(padding::vertical(5.0).horizontal(10.0)),
            rule::horizontal(1.0),
            scrollable(column(
                self.mod_browser
                    .selected_mods
                    .iter()
                    .map(|(m, r)| self.review_mod_item(m.clone(), r, state))
            ))
            .height(Length::Fill)
            .width(Length::Fill),
            rule::horizontal(1.0),
            right_center(
                row![
                    button("Back").on_press(Back).style(button::secondary),
                    button("Finish")
                        .on_press(CreateInstance)
                        .style(button::success),
                    button("Cancel").on_press(Cancel).style(button::danger)
                ]
                .height(Length::Shrink)
                .width(Length::Shrink)
                .align_y(Vertical::Center)
                .spacing(10.0)
                .padding(padding::all(10.0))
            )
            .height(Length::Shrink)
        ]
        .spacing(10.0);
        let child = if matches!(self.install_status, InstallStatus::NotStarted) {
            None
        } else {
            Some(self.view_progress_overlay())
        };
        overlay_container(base.into(), child, Some("Adding instance".into()), None)
    }
    fn view_mods<'a>(&'a self, state: &'a GruntState) -> Element<'a, Message> {
        use Message::*;
        //Main container
        column![
            self.mod_browser
                .view(self.selected_version.clone(), state)
                .map(ModBrowserMessage),
            rule::horizontal(1.0),
            right_center(
                row![
                    button("Back").on_press(Back).style(button::secondary),
                    button("Next").on_press(Next).style(button::success),
                    button("Cancel").on_press(Cancel).style(button::danger)
                ]
                .height(Length::Shrink)
                .width(Length::Shrink)
                .align_y(Vertical::Center)
                .spacing(10.0)
                .padding(padding::all(10.0))
            )
            .height(Length::Shrink)
        ]
        .into()
    }
    pub fn view<'a>(&'a self, state: &'a GruntState) -> Element<'a, Message> {
        use Message::*;
        row![
            scrollable(
                column![
                    button("1. Basics")
                        .on_press(Navigate(Step::Basic))
                        .width(Length::Fill)
                        .style(|theme, mut status| {
                            if self.step == Step::Basic {
                                status = button::Status::Pressed
                            };
                            button::subtle(theme, status)
                        }),
                    button("2. Mods")
                        .on_press_maybe(self.if_valid(Navigate(Step::Mod)))
                        .width(Length::Fill)
                        .style(|theme, mut status| {
                            if self.step == Step::Mod {
                                status = button::Status::Pressed
                            };
                            button::subtle(theme, status)
                        }),
                    button("3. Review")
                        .on_press_maybe(self.if_valid(Navigate(Step::Review)))
                        .width(Length::Fill)
                        .style(|theme, mut status| {
                            if self.step == Step::Review {
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
            match self.step {
                Step::Basic => self.view_basic(state),
                Step::Mod => self.view_mods(state),
                Step::Review => self.view_review(state),
            }
        ]
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    }
    pub fn update(&mut self, message: Message, state: &mut GruntState) -> ScreenOutput<Message> {
        use GruntAction::*;
        use Message::*;

        match message {
            ModBrowserMessage(mbm) => {
                return self
                    .mod_browser
                    .update(mbm, &mut self.selected_version, state)
                    .map(ModBrowserMessage);
            }
            Cancel => {
                return ScreenOutput::action(CloseScreen);
            }
            NameChanged(name) => {
                self.name = name;
            }
            Navigate(step) => {
                self.step = step;
            }
            Next => {
                self.step.next();
            }
            Back => {
                self.step.back();
            }
            SelectVersion(i) => {
                if let VersionCatalog::Loaded { versions } = &state.vs_versions {
                    self.selected_version = versions.get(i).cloned();
                }
            }
            RefreshVersions => {
                state.vs_versions.loading();
                return ScreenOutput::task(Task::perform(
                    refresh_versions(state.config.installations_folder.clone()),
                    VersionsLoaded,
                ));
            }

            RemoveMod(modid) => {
                self.remove_mod(modid);
            }
            VersionsLoaded(loaded_result) => match loaded_result {
                Ok(gv) => {
                    state.vs_versions.load(gv);
                    if let VersionCatalog::Loaded { versions } = &state.vs_versions {
                        self.rows = Self::version_rows(versions);
                    }
                }
                Err(e) => {
                    state.vs_versions.failed();
                    error!("Failed to load versions: {e:?}");
                }
            },
            Message::CreateInstance => {
                let Some(version) = self.selected_version.clone() else {
                    return ScreenOutput::none();
                };
                let (task, _handle) = Task::sip(
                    Self::install_instance(
                        version,
                        self.id,
                        self.mod_browser.selected_mods.clone(),
                        state.config.installations_folder.clone(),
                        state.config.instances_folder.clone(),
                    ),
                    InstanceInstalling,
                    InstanceInstalled,
                )
                .abortable();
                return ScreenOutput::task(task);
            }
            InstanceInstalled(result) => match (result, &self.selected_version) {
                (Ok((installled_path, mods)), Some(version)) => {
                    let version = version.to_local(&installled_path);
                    let name = self.name.clone();
                    return ScreenOutput::task(Task::perform(
                        instance::add_instance(
                            GruntInstance {
                                name,
                                id: self.id,
                                mods,
                                version,
                            },
                            state.config.instances_folder.clone(),
                        ),
                        InstanceCreated,
                    ));
                }
                (_, None) => {
                    error!("Could not load selected version.");
                }
                (Err(err), _) => {
                    error!("Error while trying to install instance: {}", err);
                }
            },
            InstanceCreated(result) => match result {
                Ok(instance) => {
                    return ScreenOutput::action(GruntAction::CreateInstance(instance))
                        .action_add(CloseScreen);
                }
                Err(err) => {
                    error!("An error occured while creating the instance: {}", err);
                    return ScreenOutput::none();
                }
            },
            InstanceInstalling(progress) => {
                if let InstallStatus::DownloadingMods(modid, ref modprogress) = progress {
                    self.mod_download_progress
                        .entry(modid)
                        .and_modify(|m| *m = modprogress.clone())
                        .or_insert(modprogress.clone());
                }
                self.install_status = progress;
            }
        }
        ScreenOutput::none()
    }
    fn remove_mod(&mut self, modid: i64) -> bool {
        if let Some(index) = self
            .mod_browser
            .selected_mods
            .iter()
            .position(|m| m.0.modid == modid)
        {
            self.mod_browser.selected_mods.remove(index);
            true
        } else {
            false
        }
    }
    pub fn install_instance(
        version: GameVersion,
        id: uuid::Uuid,
        mods: Vec<(Box<ModDetail>, Release)>,
        install_dir: PathBuf,
        instances_dir: PathBuf,
    ) -> impl Straw<(PathBuf, Vec<GameMod>), InstallStatus, InstallError> {
        sipper(async move |mut progress| {
            let install_path = if let GameVersionSource::Local(local_game) = version.source {
                local_game.path
            } else {
                let archive_path = download_version(version.clone(), &mut progress).await?;
                install_game(
                    version,
                    archive_path.ok_or(VersionsError::DownloadError)?,
                    install_dir,
                    &mut progress,
                )
                .await?
            };
            let instance_dir = instances_dir.join(id.to_string());
            let mod_folder = instance_dir.join("Mods");
            let logo_folder = instance_dir.join("Logos");
            let game_mods: Vec<GameMod> = stream::iter(mods)
                .map(|(mod_detail, release): (Box<ModDetail>, Release)| {
                    let modid = mod_detail.modid;
                    let releaseid = release.releaseid;
                    let mod_folder = mod_folder.clone();
                    let logo_folder = logo_folder.clone();
                    let progress = progress.clone();
                    async move {
                        let modversion = release.modversion.clone();
                        let mod_path = sipper(async move |mut mod_progress| {
                            download_mod(mod_folder, release, &mut mod_progress).await
                        })
                        .with(move |p| InstallStatus::DownloadingMods(modid, p))
                        .run(&progress)
                        .await?;
                        let logo = if let Some(logo_file) = mod_detail.logofile {
                            Some(
                                save_image(logo_folder.join(format!("{}.png", modid)), logo_file)
                                    .await?,
                            )
                        } else {
                            None
                        };
                        Ok::<GameMod, InstallError>(GameMod::moddb(
                            modid,
                            releaseid,
                            mod_path,
                            logo,
                            mod_detail.name,
                            mod_detail.text,
                            modversion,
                        ))
                    }
                })
                .buffer_unordered(3)
                .try_collect()
                .await?;
            progress.send(InstallStatus::Done).await;
            Ok((install_path, game_mods))
        })
    }
}

#[derive(Clone, Debug, Error)]
pub enum InstallError {
    #[error("Error while installing the game version: {0}")]
    VersionsError(#[from] VersionsError),
    #[error("Error while installing the Mods: {0}")]
    ModsError(#[from] ModsError),

    #[error("Error while trying to handle images: {0}")]
    ImagesError(#[from] ImagesError),
}
