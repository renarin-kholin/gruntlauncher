use std::cmp::{max, min};
use std::collections::HashSet;

use iced::{
    Element, Font, Length, Task,
    alignment::{Horizontal, Vertical},
    font, padding,
    widget::{
        Row, button, center, column, container, image, right, right_center, row, rule, scrollable,
        text, text_input,
    },
};
use iced_aw::spinner;
use iced_blitzview::web_view;

use crate::core::game_mod::GameMod;
use crate::ui::GruntAction;
use crate::{
    assets::GRUNT_ICON,
    core::version::GameVersion,
    services::game_mod::{
        ModDetail, ModDetailState, ModListEntry, ModSearchState, ModsError, Release,
        get_compatible_release, get_mod_details, search_mods,
    },
    ui::{GruntState, views::ScreenOutput, widget::release_picker},
};

pub enum Mode {
    Browsing,
    Detail { installed_release_id: i64 },
}
#[derive(Debug, Clone)]
pub enum ModNavigation {
    Next,
    Previous,
    Page(usize),
}
pub struct ModBrowser {
    pub selected_mod: Option<i64>,
    pub selected_mod_release: Option<Release>,
    pub selected_mods: Vec<(Box<ModDetail>, Release)>,
    pub mod_search_query: String,
    pub mod_search_results: ModSearchState,
    pub mod_detail: ModDetailState,
    pub mod_page_size: usize,
    pub mod_page_index: usize,
    pub mod_total: usize,
    pub requested_images: HashSet<i64>,
    pub mode: Mode,
    pub installed_mods: Vec<GameMod>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SearchChanged(String),
    Search,
    SelectMod(i64),
    OpenInBrowser(String),
    ModNavigate(ModNavigation),
    SelectModRelease(Release),
    AddMod,

    ModSearchFetched(Result<Vec<ModListEntry>, ModsError>),
    ModDetailsFetched(Result<Box<ModDetail>, ModsError>),
}
impl Default for ModBrowser {
    fn default() -> Self {
        Self::new()
    }
}

impl ModBrowser {
    pub fn new() -> Self {
        Self {
            selected_mod: None,
            selected_mod_release: None,
            selected_mods: vec![],
            mod_search_query: String::new(),
            mod_search_results: ModSearchState::NotStarted,
            mod_detail: ModDetailState::NotStarted,
            mod_page_index: 0,
            mod_page_size: 50,
            mod_total: 0,
            requested_images: HashSet::new(),
            mode: Mode::Browsing,
            installed_mods: vec![],
        }
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
        let mut item_row = row![
            container(image(mod_logo).height(50.0).width(50.0)).style(container::bordered_box),
            column![
                text!("{}", moddb_mod.name).font(Font {
                    weight: font::Weight::Bold,
                    ..Default::default()
                }),
                text!("{}", moddb_mod.author)
            ]
            .spacing(5.0)
        ]
        .padding(10.0)
        .spacing(10.0);
        if self.installed_release(moddb_mod.modid).is_some() {
            item_row = item_row.push(right_center(text!("Installed")).height(Length::Shrink));
        }
        column![
            button(item_row)
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
    pub fn view_mod_detail<'a>(
        &'a self,
        selected_version: Option<GameVersion>,
        state: &'a GruntState,
    ) -> Element<'a, Message> {
        use Message::*;
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
                    if let Some(version) = selected_version {
                        mod_release_picker = mod_release_picker.push(
                            release_picker::ReleasePicker::new(
                                &details.releases,
                                version.version,
                                selected.as_ref(),
                            )
                            .on_select(SelectModRelease),
                        )
                    }
                    if matches!(self.mode, Mode::Browsing) {
                        let in_basket = self
                            .selected_mods
                            .iter()
                            .any(|p| p.0.modid == details.modid);
                        let installed = self.installed_release(details.modid);
                        let selected_release_id =
                            self.selected_mod_release.as_ref().map(|r| r.releaseid);
                        let (label, pressable) = if in_basket {
                            ("Added", true)
                        } else if let Some(release_id) = installed {
                            if selected_release_id == Some(release_id) {
                                ("Installed", false)
                            } else {
                                ("Change Version", true)
                            }
                        } else {
                            ("Add", true)
                        };
                        mod_release_picker = mod_release_picker.push(button(label).on_press_maybe(
                            (pressable && self.selected_mod.is_some()).then_some(AddMod),
                        ));
                    }
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
                                    .style(button::subtle)
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
        mod_preview.into()
    }
    pub fn view<'a>(
        &'a self,
        selected_version: Option<GameVersion>,
        state: &'a GruntState,
    ) -> Element<'a, Message> {
        use Message::*;
        let mut mods_list = column![].height(Length::Fill);
        {
            use ModSearchState::*;
            mods_list = match &self.mod_search_results {
                NotStarted => mods_list.push(center(text!("Search results will appear here"))),
                Loading => mods_list.push(center(spinner::Spinner::new())),
                Loaded(mods) => {
                    if mods.is_empty() {
                        mods_list.push(
                            container(text!("No search results for that query")).padding(10.0),
                        )
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

        let mut mod_view = row![];
        if let Mode::Browsing = self.mode {
            mod_view = mod_view
                .push(
                    column![
                        column![
                            text!("Search"),
                            row![
                                text_input("Search for mods", &self.mod_search_query)
                                    .on_input(SearchChanged),
                                button("Search").on_press(Search).style(
                                    move |theme, mut status| {
                                        if self.mod_search_query.len() < 4 {
                                            status = button::Status::Disabled;
                                        }
                                        button::primary(theme, status)
                                    }
                                )
                            ]
                            .spacing(10.0)
                        ]
                        .padding(padding::all(10.0))
                        .spacing(5.0),
                        rule::horizontal(1.0),
                        mods_list
                    ]
                    .width(Length::FillPortion(2)),
                )
                .push(rule::vertical(1.0))
        };
        mod_view
            .push(
                //Search mods
                self.view_mod_detail(selected_version, state),
            )
            .into()
    }

    pub fn update(
        &mut self,
        message: Message,
        selected_version: &mut Option<GameVersion>,
        state: &mut GruntState,
    ) -> ScreenOutput<Message> {
        use Message::*;
        match message {
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

                    let compatible_release = match &self.mode {
                        Mode::Browsing => selected_version.as_ref().map(|gameversion| {
                            get_compatible_release(&mod_details.releases, gameversion)
                        }),
                        Mode::Detail {
                            installed_release_id,
                        } => mod_details
                            .releases
                            .iter()
                            .find(|r| r.releaseid == *installed_release_id)
                            .cloned()
                            .or_else(|| {
                                // Installed release no longer listed on ModDB.
                                selected_version.as_ref().map(|gameversion| {
                                    get_compatible_release(&mod_details.releases, gameversion)
                                })
                            }),
                    };
                    self.selected_mod_release = compatible_release;
                    self.mod_detail = ModDetailState::Loaded(mod_details);
                }
                Err(e) => {
                    self.mod_detail = ModDetailState::Failed(e);
                }
            },
        }
        ScreenOutput::none()
    }
    pub fn mod_version_changed(&self, original_release_id: i64) -> bool {
        if let Some(mod_release) = &self.selected_mod_release {
            mod_release.releaseid != original_release_id
        } else {
            false
        }
    }
    pub fn mod_list_changed(&self, original: &[GameMod]) -> bool {
        self.selected_mods
            .iter()
            .any(|(detail, release)| !Self::is_installed(original, detail.modid, release.releaseid))
    }
    fn is_installed(mods: &[GameMod], modid: i64, releaseid: i64) -> bool {
        mods.iter().any(|m| {
            matches!(
                m.source,
                crate::core::game_mod::ModSource::ModDb { mod_id, release_id, .. }
                    if mod_id == modid && release_id == releaseid
            )
        })
    }
    pub fn picks(&self) -> Vec<(Box<ModDetail>, Release)> {
        match self.mode {
            Mode::Browsing => self.selected_mods.clone(),
            Mode::Detail {
                installed_release_id,
            } => match (&self.mod_detail, &self.selected_mod_release) {
                (ModDetailState::Loaded(detail), Some(release))
                    if release.releaseid != installed_release_id =>
                {
                    vec![(detail.clone(), release.clone())]
                }
                _ => vec![],
            },
        }
    }
    fn installed_release(&self, modid: i64) -> Option<i64> {
        self.installed_mods.iter().find_map(|m| match m.source {
            crate::core::game_mod::ModSource::ModDb {
                mod_id, release_id, ..
            } if mod_id == modid => Some(release_id),
            _ => None,
        })
    }
    fn remove_mod(&mut self, modid: i64) -> bool {
        if let Some(index) = self.selected_mods.iter().position(|m| m.0.modid == modid) {
            self.selected_mods.remove(index);
            true
        } else {
            false
        }
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
}
