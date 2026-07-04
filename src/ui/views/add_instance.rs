use std::{
    cmp::{max, min},
    collections::HashSet,
    path::PathBuf,
};

use iced::{
    Element,
    Length::{self, Fill, Shrink},
    Task,
    alignment::{Horizontal, Vertical},
    padding,
    task::{Straw, sipper},
    widget::{
        Row, button, center, column, container, image, progress_bar, right, right_center, row,
        rule, scrollable, text, text_input,
    },
};
use iced_aw::spinner;
use iced_blitzview::web_view;
use tracing::{debug, error};

use crate::{
    assets::GRUNT_ICON,
    core::{
        instance::GruntInstance,
        version::{GameVersion, GameVersionSource, VersionCatalog},
    },
    services::{
        game_mod::{
            ModDetail, ModDetailState, ModListEntry, ModSearchState, ModsError, Release,
            get_compatible_release, get_mod_details, search_mods,
        },
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
            release_picker,
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
    selected_mod: Option<i64>,
    selected_mod_release: Option<Release>,
    selected_mods: Vec<(Box<ModDetail>, Release)>,
    install_progress: InstallProgress,
    mod_search_query: String,
    mod_search_results: ModSearchState,
    mod_detail: ModDetailState,
    mod_page_size: usize,
    mod_page_index: usize,
    mod_total: usize,
    requested_images: HashSet<i64>,
}
#[derive(Debug, Clone)]
pub enum ModNavigation {
    Next,
    Previous,
    Page(usize),
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
    SearchChanged(String),
    Search,
    SelectMod(i64),
    OpenInBrowser(String),
    ModNavigate(ModNavigation),
    SelectModRelease(Release),
    AddMod,

    //Review Step events
    CreateInstance,
    RemoveMod(i64),

    //Webview events
    // ModViewPageFetched(Result<String, String>),

    //Service Result events
    VersionsLoaded(Result<Vec<GameVersion>, VersionsError>),
    VersionInstalling(InstallProgress),
    VersionInstalled(Result<PathBuf, VersionsError>),
    ModSearchFetched(Result<Vec<ModListEntry>, ModsError>),
    ModDetailsFetched(Result<Box<ModDetail>, ModsError>),
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
            selected_mod_release: None,
            selected_mods: vec![],
            mod_search_query: String::new(),
            mod_search_results: ModSearchState::NotStarted,
            mod_detail: ModDetailState::NotStarted,
            mod_page_index: 0,
            mod_page_size: 50,
            mod_total: 0,
            requested_images: HashSet::new(),
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
                    table::Table::new(&self.columns, &self.rows)
                        .row_height(30.0)
                        .on_select(SelectVersion),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(padding::all(1.0))
                .style(container::bordered_box),
            )
        };
        let next_if_valid = if !self.name.is_empty() && self.selected_version.is_some() {
            Some(Next)
        } else {
            None
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
            right(
                row![
                    button("Next")
                        .on_press_maybe(next_if_valid)
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
        let mut mod_logo = image::Handle::from_bytes(GRUNT_ICON);
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
    fn view_review(&self, state: &GruntState) -> Element<'_, Message> {
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
            scrollable(column(
                self.selected_mods
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
        let child = if matches!(self.install_progress, InstallProgress::NotStarted) {
            None
        } else {
            Some(self.view_progress_overlay())
        };
        overlay_container(base.into(), child, Some("Adding instance".into()))
    }
    fn mod_item<'a>(
        &'a self,
        state: &GruntState,
        moddb_mod: &'a ModListEntry,
    ) -> Element<'a, Message> {
        use Message::*;
        let mut mod_logo = image::Handle::from_bytes(GRUNT_ICON);
        if let Some(logo) = state.image_cache.peek(&moddb_mod.modid) {
            mod_logo = logo.clone();
        }
        column![
            button(
                row![
                    container(image(mod_logo).height(50.0).width(50.0))
                        .style(container::bordered_box),
                    column![text!("{}", moddb_mod.name), text!("{}", moddb_mod.author)]
                        .spacing(5.0)
                ]
                .padding(10.0)
                .spacing(10.0),
            )
            .on_press(SelectMod(moddb_mod.modid))
            .style(move |theme, mut status| {
                if let Some(s) = self.selected_mod
                    && s == moddb_mod.modid
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
        let mut mods_list = column![].height(Length::Fill);
        {
            use ModSearchState::*;
            mods_list = match &self.mod_search_results {
                NotStarted => mods_list,
                Loading => mods_list.push(center(spinner::Spinner::new())),
                Loaded(mods) => {
                    if mods.is_empty() {
                        mods_list.push(text!("No search results for that query"))
                    } else {
                        let mods_list = mods_list.push(
                            scrollable(column(
                                mods.iter()
                                    .skip(self.mod_page_size * self.mod_page_index)
                                    .take(self.mod_page_size)
                                    .map(|m| self.mod_item(state, m)),
                            ))
                            .height(Length::Fill),
                        );

                        if self.mod_total > self.mod_page_size {
                            use ModNavigation::*;
                            let mut pagination =
                                row![button("Prev").on_press(ModNavigate(Previous))]
                                    .width(Length::Fill)
                                    .spacing(10.0)
                                    .padding(10.0);
                            let n_pages = self.mod_total.div_ceil(self.mod_page_size);
                            let last = n_pages.saturating_sub(1);
                            for x in self.mod_page_index..=min(self.mod_page_index + 3, last - 1) {
                                pagination = pagination.push(
                                    button(text!("{}", x + 1))
                                        .style(move |theme, mut status| {
                                            if self.mod_page_index == x {
                                                status = button::Status::Pressed;
                                            }
                                            button::subtle(theme, status)
                                        })
                                        .on_press(ModNavigate(Page(x))),
                                );
                            }
                            pagination = pagination.push(
                                right(
                                    button(text!("{}", last + 1))
                                        .style(button::subtle)
                                        .on_press(ModNavigate(Page(last))),
                                )
                                .align_x(Horizontal::Right),
                            );
                            pagination =
                                pagination.push(right(button("Next").on_press(ModNavigate(Next))));
                            mods_list.push(container(pagination).style(container::bordered_box))
                        } else {
                            mods_list
                        }
                    }
                }
                Failed(e) => {
                    mods_list.push(text!("There was an error while trying to load mods: {e}"))
                }
            };
        }

        let mut mod_preview = column![].width(Length::FillPortion(3));

        {
            use ModDetailState::*;
            mod_preview = match &self.mod_detail {
                NotStarted => mod_preview.push(center(text!("Select a mod to show its preview."))),
                Loading => mod_preview.push(center(
                    column![
                        text!("Loading mod details"),
                        spinner::Spinner::default().width(30.0).height(30.0)
                    ]
                    .spacing(10.0)
                    .align_x(Horizontal::Center),
                )),
                Loaded(details) => {
                    let page_url = format!(
                        "https://mods.vintagestory.at/{}",
                        if let Some(urlalias) = &details.urlalias {
                            urlalias.clone()
                        } else {
                            format!("show/{}", details.modid).to_string()
                        }
                    );
                    let mut mod_release_picker: Row<'_, Message> = row![]
                        .spacing(10.0)
                        .padding(padding::all(10.0))
                        .align_y(Vertical::Center);
                    let selected = &self.selected_mod_release;
                    if let Some(version) = &self.selected_version {
                        mod_release_picker = mod_release_picker.push(
                            release_picker::ReleasePicker::new(
                                &details.releases,
                                &version.version,
                                selected.as_ref(),
                            )
                            .on_select(SelectModRelease),
                        )
                    }
                    mod_release_picker = mod_release_picker.push(
                        button(
                            if self
                                .selected_mods
                                .iter()
                                .find(|p| p.0.modid == details.modid)
                                .is_some()
                            {
                                "Added"
                            } else {
                                "Add"
                            },
                        )
                        .on_press_maybe(if self.selected_mod.is_some() {
                            Some(AddMod)
                        } else {
                            None
                        }),
                    );
                    mod_preview.push(column![
                        column!(text!("{}", details.name))
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
                                    .on_press(OpenInBrowser(page_url))
                            ]
                            .align_y(Vertical::Center)
                            .padding(10.0),
                            right_center(mod_release_picker).height(Length::Shrink)
                        ]
                        .height(Length::Shrink),
                    ])
                }
                Failed(e) => mod_preview.push(text!("Failed trying to load mod details: {}", e)),
            };
        }
        //Main container
        column![
            row![
                //Search mods
                column![
                    column![
                        text!("Search"),
                        row![
                            text_input("Search for mods", &self.mod_search_query)
                                .on_input(SearchChanged),
                            button("Search")
                                .on_press(Search)
                                .style(move |theme, mut status| {
                                    if self.mod_search_query.len() < 4 {
                                        status = button::Status::Disabled;
                                    }
                                    button::primary(theme, status)
                                })
                        ]
                        .spacing(10.0)
                    ]
                    .padding(padding::all(10.0))
                    .spacing(5.0),
                    rule::horizontal(1.0),
                    mods_list
                ]
                .width(Length::FillPortion(2)),
                rule::vertical(1.0),
                mod_preview
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

            SelectMod(i) => {
                self.selected_mod = Some(i);
                self.mod_detail = ModDetailState::Loading;
                // return ScreenOutput::task(Self::fetch_mod(format!("{}mod/{}", VSMODDB, i)));
                return ScreenOutput::task(Task::perform(
                    get_mod_details(i.to_string()),
                    ModDetailsFetched,
                ));
            }
            SelectModRelease(release) => {
                self.selected_mod_release = Some(release);
            }
            AddMod => {
                if let (ModDetailState::Loaded(selected_mod), Some(selected_release)) =
                    (self.mod_detail.clone(), self.selected_mod_release.clone())
                    && !self.remove_mod(selected_mod.modid)
                {
                    self.selected_mods
                        .push((selected_mod.clone(), selected_release));
                }
            }
            RemoveMod(modid) => {
                self.remove_mod(modid);
            }
            SearchChanged(query) => {
                self.mod_search_query = query;
            }
            Search => {
                self.mod_search_results = ModSearchState::Loading;
                return ScreenOutput::task(Task::perform(
                    search_mods(self.mod_search_query.clone()),
                    ModSearchFetched,
                ));
            }
            ModSearchFetched(result) => match result {
                Ok(mods) => {
                    self.mod_total = mods.len();
                    self.mod_page_size = 50;
                    self.mod_page_index = 0;
                    self.mod_search_results = ModSearchState::Loaded(mods);
                    return self.fetch_current_page_images(state);
                }
                Err(e) => self.mod_search_results = ModSearchState::Failed(e),
            },
            ModDetailsFetched(result) => match result {
                Ok(mod_details) => {
                    state.webview_content.load_html(&mod_details.text);

                    let compatible_release = self.selected_version.as_ref().map(|gameversion| {
                        get_compatible_release(&mod_details.releases, gameversion)
                    });
                    self.selected_mod_release = compatible_release;
                    self.mod_detail = ModDetailState::Loaded(mod_details);
                }
                Err(e) => {
                    self.mod_detail = ModDetailState::Failed(e);
                }
            },
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
                    // No version selected yet — form validation will surface this later.
                    return ScreenOutput::none();
                };
                // TODO(#1): persist the instance (GruntAction::CreateInstance) once the
                // download/extract flow is wired up.
                let (task, _handle) = Task::sip(
                    Self::install_version(version, state.config.installations_folder.clone()),
                    VersionInstalling,
                    VersionInstalled,
                )
                .abortable();
                return ScreenOutput::task(task);
            }
            VersionInstalled(result) => {
                debug!("{:?}", result);
                if let (Ok(installled_path), Some(version)) =
                    (result, self.selected_version.clone())
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
                            state.config.instances_folder.clone(),
                        ),
                        InstanceCreated,
                    ));
                } else {
                    return ScreenOutput::none();
                }
            }
            InstanceCreated(result) => {
                if let Ok(instance) = result {
                    return ScreenOutput::action(GruntAction::CreateInstance(instance))
                        .action_add(CloseScreen);
                } else {
                    return ScreenOutput::none();
                }
            }
            VersionInstalling(progress) => {
                self.install_progress = progress;
            }
            OpenInBrowser(url) => {
                let _ = webbrowser::open(&url);
            }
            ModNavigate(navigation) => {
                use ModNavigation::*;
                let original = self.mod_page_index;
                let n_pages = self.mod_total.div_ceil(self.mod_page_size);
                let last = n_pages.saturating_sub(1);
                match navigation {
                    Next => {
                        self.mod_page_index = min(self.mod_page_index + 1, last);
                    }
                    Previous => {
                        self.mod_page_index = max(self.mod_page_index - 1, 0);
                    }
                    Page(page) => {
                        self.mod_page_index = page;
                    }
                }
                if self.mod_page_index != original {
                    return self.fetch_current_page_images(state);
                }
            }
        }
        ScreenOutput::none()
    }
    fn fetch_current_page_images(&mut self, state: &GruntState) -> ScreenOutput<Message> {
        let ModSearchState::Loaded(mods) = &self.mod_search_results else {
            return ScreenOutput::none();
        };
        let mut output = ScreenOutput::none();
        for m in mods
            .iter()
            .skip(self.mod_page_size * self.mod_page_index)
            .take(self.mod_page_size)
        {
            if state.image_cache.peek(&m.modid).is_some() {
                continue;
            } else {
                if let Some(logo) = &m.logo
                    && self.requested_images.insert(m.modid)
                {
                    output = output.action_add(GruntAction::GetImage {
                        id: m.modid,
                        url: logo.to_string(),
                    })
                }
            }
        }
        output
    }
    fn remove_mod(&mut self, modid: i64) -> bool {
        if let Some(index) = self.selected_mods.iter().position(|m| m.0.modid == modid) {
            self.selected_mods.remove(index);
            true
        } else {
            false
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
