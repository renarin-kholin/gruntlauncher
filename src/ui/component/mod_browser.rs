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
        column![
            button(
                row![
                    container(image(mod_logo).height(50.0).width(50.0))
                        .style(container::bordered_box),
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
    pub fn view<'a>(
        &'a self,
        selected_version: &'a Option<GameVersion>,
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
        //Main container
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
        ]
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

                    let compatible_release = selected_version.as_ref().map(|gameversion| {
                        get_compatible_release(&mod_details.releases, gameversion)
                    });
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
