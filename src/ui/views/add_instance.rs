use std::path::PathBuf;

use iced::{
    Element,
    Length::{self, Fill, Shrink},
    Task,
    alignment::{Horizontal, Vertical},
    padding,
    task::{Straw, sipper},
    widget::{
        button, center, column, container, image, progress_bar, right_center, row, rule,
        scrollable, text, text_input,
    },
};
use iced_aw::spinner;
use iced_blitzview::web_view;
use tracing::{debug, error};

use crate::{
    core::{
        instance::GruntInstance,
        version::{GameVersion, GameVersionSource, VersionCatalog},
    },
    services::{
        instance::{self, InstancesError},
        version::{
            InstallProgress, VersionsError, download_version, extract_archive, load_versions,
            refresh_versions,
        },
    },
    ui::{
        GruntAction, GruntState,
        views::ScreenOutput,
        widget::{
            overlay::overlay_container,
            table::{self, TableColumn},
        },
    },
};

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
    name: String,
    selected_version: Option<GameVersion>,
    columns: Vec<TableColumn>,
    rows: Vec<Vec<String>>,
    step: Step,
    selected_mod: Option<usize>,
    install_progress: InstallProgress,
}

#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    SelectMod(usize),
    SelectVersion(usize),
    RefreshVersions,
    Navigate(Step),
    OpenInBrowser(String),
    Next,
    Back,
    Cancel,
    CreateInstance,

    //Webview events
    ModViewPageFetched(Result<String, String>),

    //Service Result events
    VersionsLoaded(Result<Vec<GameVersion>, VersionsError>),
    VersionInstalling(InstallProgress),
    VersionInstalled(Result<PathBuf, VersionsError>),

    InstanceCreated(Result<GruntInstance, InstancesError>),
}

impl Screen {
    pub fn new(state: &mut GruntState) -> (Self, Task<Message>) {
        let mut screen = Self {
            name: String::new(),
            selected_version: None,
            columns: vec![
                TableColumn::new("Version", 150.0).min_width(80.0),
                TableColumn::new("Type", 300.0).min_width(80.0),
            ],
            rows: vec![],
            step: Step::Basic,
            selected_mod: None,
            install_progress: InstallProgress::NotStarted,
        };

        // Reuse an already-loaded catalog; otherwise kick off a load.
        let task = match &state.vs_versions {
            VersionCatalog::Loaded { versions } => {
                screen.rows = Self::version_rows(versions);
                Task::none()
            }
            _ => {
                state.vs_versions.loading();
                if let Some(config) = state.config.clone() {
                    Task::perform(
                        load_versions(config.installations_folder),
                        Message::VersionsLoaded,
                    )
                } else {
                    Task::none()
                }
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
                scrollable(
                    container(
                        table::Table::new(&self.columns, &self.rows)
                            .row_height(30.0)
                            .on_select(SelectVersion),
                    )
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(padding::all(1.0))
                    .style(container::bordered_box),
                )
                .height(Length::Fill)
                .width(Length::Fill)
                .style(|theme, status| scrollable::Style {
                    container: container::bordered_box(theme),
                    ..scrollable::default(theme, status)
                }),
            )
        };
        column![
            //Instance details (name and icon)
            row![
                button(image("assets/icons/logo.png").height(50.0).width(50.0)).style(button::text),
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
            row![
                button("Next").on_press(Next).style(button::success),
                button("Cancel").on_press(Cancel).style(button::danger)
            ]
            .spacing(10.0)
            .align_y(Vertical::Center)
        ]
        .spacing(10.0)
        .padding(padding::all(10.0))
        .align_x(Horizontal::Right)
        .into()
    }
    fn review_mod_item(&self, i: usize) -> Element<'_, Message> {
        use Message::*;
        column![
            button(
                row![
                    container(image("assets/icons/logo.png").height(50.0).width(50.0))
                        .style(container::bordered_box),
                    column![text!("Mod name"), text!("Mod Version")].spacing(5.0),
                    right_center(
                        row![button("Delete")]
                            .spacing(5.0)
                            .align_y(Vertical::Center)
                    )
                ]
                .padding(10.0)
                .spacing(10.0),
            )
            .on_press(SelectMod(i))
            .style(button::subtle)
            .width(Length::Fill),
            rule::horizontal(1.0)
        ]
        .into()
    }
    fn view_progress_overlay(&self) -> Element<'_, Message> {
        use InstallProgress::*;

        let main_container = column![].spacing(10.0).padding(10.0);
        match &self.install_progress {
            NotStarted => main_container.push(text!("Starting download please wait...")),
            Downloading { downloaded, total } => main_container
                .push(text!("Downloading game file"))
                .push(progress_bar(0.0..=(*total as f32), *downloaded as f32)),
            Verifying => main_container.push(text!("Verifying the download hash...")),
            Installing => main_container
                .push(text!("Installing"))
                .push(progress_bar(0.0..=100.0, 25.0)),
            Done => main_container.push(text!("Finished installing.")),
            Failed(e) => main_container.push(text!("{}", e)),
        }
        .into()
    }
    fn view_review(&self) -> Element<'_, Message> {
        use Message::*;

        let base = column![
            //Instance details (name and icon)
            row![
                button(image("assets/icons/logo.png").height(50.0).width(50.0)).style(button::text),
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
            scrollable(column((0..100).map(|i| self.review_mod_item(i))))
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
        let child = if matches!(self.install_progress, InstallProgress::NotStarted) {
            None
        } else {
            Some(self.view_progress_overlay())
        };
        overlay_container(base.into(), child, Some("Adding instance".into()))
    }
    fn mod_item(&self, i: usize) -> Element<'_, Message> {
        use Message::*;
        column![
            button(
                row![
                    container(image("assets/icons/logo.png").height(50.0).width(50.0))
                        .style(container::bordered_box),
                    column![text!("Mod name"), text!("Mod Version")].spacing(5.0)
                ]
                .padding(10.0)
                .spacing(10.0),
            )
            .on_press(SelectMod(i))
            .style(move |theme, mut status| {
                if let Some(s) = self.selected_mod
                    && s == i
                {
                    status = button::Status::Pressed
                };
                button::Style {
                    ..button::subtle(theme, status)
                }
            })
            .width(Length::Fill),
            rule::horizontal(1.0)
        ]
        .into()
    }
    fn view_mods<'a>(&'a self, state: &'a GruntState) -> Element<'a, Message> {
        use Message::*;
        //Main container
        column![
            row![
                //Search mods
                column![
                    column![text!("Search"), text_input("Search for mods", "")]
                        .padding(padding::all(10.0))
                        .spacing(5.0),
                    rule::horizontal(1.0),
                    scrollable(column((0..100).map(|i| self.mod_item(i))))
                ]
                .width(Length::FillPortion(2)),
                rule::vertical(1.0),
                //View mods info and select
                column![
                    column!(text!("Mod Name"))
                        .spacing(5.0)
                        .padding(padding::all(10.0)),
                    rule::horizontal(1.0),
                    //Mod info
                    scrollable(web_view(&state.webview_content)).height(Length::Fill),
                    rule::horizontal(1.0),
                    row![
                        row![
                            button("Open in default browser")
                                .style(button::text)
                                .on_press(OpenInBrowser(
                                    "https://mods.vintagestory.at/algernonswatersheds".to_string()
                                ))
                        ]
                        .align_y(Vertical::Center)
                        .padding(10.0),
                        right_center(
                            row![text!("Version dropdown"), button("Add")]
                                .spacing(10.0)
                                .padding(padding::all(10.0))
                                .align_y(Vertical::Center)
                        )
                        .height(Length::Shrink)
                    ]
                    .height(Length::Shrink)
                ]
                .width(Length::FillPortion(3))
            ],
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
                        .on_press(Navigate(Step::Mod))
                        .width(Length::Fill)
                        .style(|theme, mut status| {
                            if self.step == Step::Mod {
                                status = button::Status::Pressed
                            };
                            button::subtle(theme, status)
                        }),
                    button("3. Review")
                        .on_press(Navigate(Step::Review))
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
                Step::Review => self.view_review(),
            }
        ]
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    }
    fn fetch_mod(url: String) -> Task<Message> {
        iced::Task::future(async move {
            let client = reqwest::Client::new();
            let response: serde_json::Value = client
                .get(url)
                .header("Accept", "application/json")
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            let html = response["mod"]["text"].as_str().unwrap();
            Message::ModViewPageFetched(Ok(html.to_string()))
        })
    }
    pub fn update(&mut self, message: Message, state: &mut GruntState) -> ScreenOutput<Message> {
        use GruntAction::*;
        use Message::*;

        match message {
            Cancel => ScreenOutput::action(CloseScreen),
            NameChanged(name) => {
                self.name = name;
                ScreenOutput::none()
            }
            Navigate(step) => {
                self.step = step;
                ScreenOutput::none()
            }
            Next => {
                self.step.next();
                ScreenOutput::none()
            }
            Back => {
                self.step.back();
                ScreenOutput::none()
            }
            SelectVersion(i) => {
                if let VersionCatalog::Loaded { versions } = &state.vs_versions {
                    self.selected_version = versions.get(i).cloned();
                }
                ScreenOutput::none()
            }
            RefreshVersions => {
                state.vs_versions.loading();
                if let Some(config) = state.config.clone() {
                    return ScreenOutput::task(Task::perform(
                        refresh_versions(config.installations_folder),
                        VersionsLoaded,
                    ));
                }
                ScreenOutput::none()
            }

            SelectMod(i) => {
                self.selected_mod = Some(i);
                ScreenOutput::task(Self::fetch_mod(
                    "https://mods.vintagestory.at/api/mod/7286".to_string(),
                ))
            }
            VersionsLoaded(loaded_result) => {
                match loaded_result {
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
                }
                ScreenOutput::none()
            }
            Message::CreateInstance => {
                let Some(version) = self.selected_version.clone() else {
                    // No version selected yet — form validation will surface this later.
                    return ScreenOutput::none();
                };
                if let Some(config) = &state.config {
                    // TODO(#1): persist the instance (GruntAction::CreateInstance) once the
                    // download/extract flow is wired up.
                    let (task, _handle) = Task::sip(
                        Self::install_version(version, config.installations_folder.clone()),
                        VersionInstalling,
                        VersionInstalled,
                    )
                    .abortable();
                    ScreenOutput::task(task)
                } else {
                    ScreenOutput::none()
                }
            }
            VersionInstalled(result) => {
                debug!("{:?}", result);
                if let (Ok(installled_path), Some(config), Some(version)) =
                    (result, state.config.clone(), self.selected_version.clone())
                {
                    let version = version.to_local(&installled_path);
                    let name = self.name.clone();
                    return ScreenOutput::task(Task::perform(
                        instance::add_instance(
                            GruntInstance {
                                name,
                                id: uuid::Uuid::new_v4(),
                                mods: vec![],
                                version,
                            },
                            config.instances_folder,
                        ),
                        InstanceCreated,
                    ));
                }
                ScreenOutput::none()
            }
            InstanceCreated(result) => {
                if let Ok(instance) = result {
                    return ScreenOutput::action(GruntAction::CreateInstance(instance))
                        .action_add(CloseScreen);
                }
                ScreenOutput::none()
            }
            VersionInstalling(progress) => {
                self.install_progress = progress;
                ScreenOutput::none()
            }
            ModViewPageFetched(Ok(page)) => {
                state.webview_content.load_html(&page);
                ScreenOutput::none()
            }
            OpenInBrowser(url) => {
                let _ = webbrowser::open(&url);
                ScreenOutput::none()
            }
            _ => ScreenOutput::none(),
        }
    }
    pub fn install_version(
        version: GameVersion,
        install_dir: PathBuf,
    ) -> impl Straw<PathBuf, InstallProgress, VersionsError> {
        sipper(async move |mut progress| {
            let install_path = if let GameVersionSource::Local(local_game) = version.source {
                local_game.path
            } else {
                let archive_path = download_version(version.clone(), &mut progress).await?;
                extract_archive(
                    version,
                    archive_path.ok_or(VersionsError::DownloadError)?,
                    install_dir,
                    &mut progress,
                )
                .await?
            };
            progress.send(InstallProgress::Done).await;
            Ok(install_path)
        })
    }
}
