use crate::assets::GRUNT_ICON;
use crate::core::game_mod::{GameMod, ModSource};
use crate::core::instance::GruntInstance;
use crate::core::version::{GameVersion, VersionCatalog};
use crate::services::instance::{InstancesError, save_instance};
use crate::services::version::{VersionsError, load_versions};
use crate::ui::views::ScreenOutput;
use crate::ui::widget::overlay::overlay_container;
use crate::ui::widget::table::{self, TableColumn};
use crate::ui::{GruntAction, GruntState};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{center, container, right, right_center, space, text_input};
use iced::{
    Element, Length, Task, padding,
    widget::{button, column, image, row, rule, scrollable, text},
};
use iced_aw::spinner;
use std::collections::HashSet;
use tracing::error;

#[derive(Debug, Clone)]
pub enum Message {
    Navigate(Tab),

    //Form element events
    NameChanged(String),
    SelectVersion(usize),
    DiscardChanges,
    SaveChanges,
    RemoveMod(i64),

    VersionsLoaded(Result<Vec<GameVersion>, VersionsError>),
    InstanceSaved(Result<(), InstancesError>),
}
#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    General,
    Mods,
}
pub struct Screen {
    selected_tab: Tab,
    instance: GruntInstance,
    changed: bool,
    version_columns: Vec<TableColumn>,
    version_rows: Vec<Vec<String>>,
    icon_handle: image::Handle,
    requested_images: HashSet<i64>,
}
impl Screen {
    fn version_rows(versions: &[GameVersion]) -> Vec<Vec<String>> {
        versions
            .iter()
            .map(|v| vec![v.version.to_string(), "Release".to_string()])
            .collect()
    }
    pub fn new(state: &mut GruntState, instance: GruntInstance) -> (Self, Task<Message>) {
        let mut screen = Self {
            selected_tab: Tab::General,
            instance,
            changed: false,
            version_columns: vec![
                TableColumn::new("Version", 150.0).min_width(80.0),
                TableColumn::new("Type", 300.0).min_width(80.0),
            ],
            version_rows: vec![],
            icon_handle: image::Handle::from_bytes(GRUNT_ICON),
            requested_images: HashSet::new(),
        };
        // Reuse an already-loaded catalog; otherwise kick off a load.
        let task = match &state.vs_versions {
            VersionCatalog::Loaded { versions } => {
                screen.version_rows = Self::version_rows(versions);
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
    pub fn changed_maybe(&self, m: Message) -> Option<Message> {
        if self.changed { Some(m) } else { None }
    }
    pub fn view_general<'a>(&'a self, state: &'a GruntState) -> Element<'a, Message> {
        use Message::*;
        let mut version_selector = column![];
        version_selector = if matches!(state.vs_versions, VersionCatalog::Loading) {
            version_selector.push(
                center(
                    column![
                        text!("Loading Versions"),
                        spinner::Spinner::default().width(30.0).height(30.0)
                    ]
                    .align_x(Horizontal::Center)
                    .height(Length::Shrink)
                    .spacing(10.0),
                )
                .height(Length::Fill)
                .width(Length::Fill),
            )
        } else {
            version_selector.push(
                container(
                    table::Table::new(
                        &self.version_columns,
                        &self.version_rows,
                        self.version_rows
                            .iter()
                            .position(|r| r[0] == self.instance.version.version.to_string()),
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
            text!("Instance Name"),
            text_input("Name", &self.instance.name).on_input(NameChanged),
            space().height(10.0),
            text!("Edit version. Note that this might cause issues with existing game worlds."),
            version_selector,
            space().height(10.0),
            right(
                row![
                    button("Discard Changes").on_press_maybe(self.changed_maybe(DiscardChanges)),
                    button("Save").on_press_maybe(self.changed_maybe(SaveChanges))
                ]
                .align_y(Vertical::Center)
                .spacing(10.0)
            )
        ]
        .spacing(5.0)
        .padding(10.0)
        .into()
    }

    pub fn view_mod_item<'a>(
        &'a self,
        game_mod: &GameMod,
        state: &'a GruntState,
    ) -> Element<'a, Message> {
        use Message::*;
        let mut mod_logo = self.icon_handle.clone();
        if let ModSource::ModDb {
            mod_id,
            name,
            version,
            ..
        } = &game_mod.source
        {
            if let Some(logo) = state.image_cache.peek(mod_id) {
                mod_logo = logo.clone();
            }

            column![
                button(
                    row![
                        container(image(mod_logo).height(50.0).width(50.0))
                            .style(container::bordered_box),
                        column![text!("{}", name), text!("{}", version.to_string())].spacing(5.0),
                        right_center(
                            row![
                                button("Change Version"),
                                button("Remove").on_press(RemoveMod(*mod_id))
                            ]
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
        } else {
            space().into()
        }
    }
    pub fn view_mods<'a>(&'a self, state: &'a GruntState) -> Element<'a, Message> {
        use Message::*;
        column![
            column![text!("Mods")].padding(10.0),
            scrollable(column(
                self.instance
                    .mods
                    .iter()
                    .map(|m| self.view_mod_item(m, state))
            ))
            .height(Length::Fill)
            .width(Length::Fill),
            space().height(10.0),
            row![
                button("Add Mod"),
                right(
                    row![
                        button("Discard Changes")
                            .on_press_maybe(self.changed_maybe(DiscardChanges)),
                        button("Save").on_press_maybe(self.changed_maybe(SaveChanges))
                    ]
                    .spacing(10.0)
                    .align_y(Vertical::Center)
                )
            ]
            .align_y(Vertical::Center)
            .spacing(10.0)
            .padding(10.0)
        ]
        .spacing(5.0)
        .into()
    }
    pub fn view<'a>(&'a self, state: &'a GruntState) -> Element<'a, Message> {
        use Message::*;
        //TODO: Extract this into a reusable widget
        let base = row![
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
                    button("Mods")
                        .on_press(Navigate(Tab::Mods))
                        .width(Length::Fill)
                        .style(|theme, mut status| {
                            if self.selected_tab == Tab::Mods {
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
                Tab::Mods => self.view_mods(state),
            }
        ]
        .height(Length::Fill)
        .width(Length::Fill);

        overlay_container(base.into(), None, None, None)
    }
    pub fn update(&mut self, message: Message, state: &mut GruntState) -> ScreenOutput<Message> {
        use Message::*;
        match message {
            Navigate(tab) => {
                self.selected_tab = tab.clone();
                if matches!(tab, Tab::Mods) {
                    return self.fetch_current_page_images(state);
                }
            }
            NameChanged(name) => {
                self.instance.name = name;
                self.on_changed(state);
            }
            SelectVersion(i) => {
                if let VersionCatalog::Loaded { versions } = &state.vs_versions
                    && let Some(version) = versions.get(i).cloned()
                {
                    self.instance.version = version;
                    self.on_changed(state);
                }
            }
            RemoveMod(id) => {
                self.instance.mods.retain(|m| {
                    if let ModSource::ModDb { mod_id, .. } = m.source {
                        mod_id != id
                    } else {
                        //TODO: support working with local mods later
                        true
                    }
                });
                self.on_changed(state);
            }
            DiscardChanges => match self.selected_tab {
                Tab::General => self.reset_general(state),
                Tab::Mods => self.reset_mods(state),
            },
            SaveChanges => {
                match self.selected_tab {
                    Tab::General => {
                        self.save(state);
                    }
                    Tab::Mods => {}
                }
                return ScreenOutput::task(Task::perform(
                    save_instance(self.instance.clone(), state.config.instances_folder.clone()),
                    InstanceSaved,
                ));
            }
            InstanceSaved(result) => match result {
                Ok(()) => {
                    self.changed = false;
                }
                Err(e) => {
                    error!("{e}");
                }
            },
            VersionsLoaded(load_result) => match load_result {
                Ok(gv) => {
                    state.vs_versions.load(gv);
                    if let VersionCatalog::Loaded { versions } = &state.vs_versions {
                        self.version_rows = Self::version_rows(versions);
                    }
                }
                Err(e) => {
                    state.vs_versions.failed();
                    error!("Failed to load versions: {e:?}");
                }
            },
        }
        ScreenOutput::none()
    }
    fn reset_general(&mut self, state: &GruntState) {
        if let Some(original) = state.instances.iter().find(|i| i.id == self.instance.id) {
            self.instance.name = original.name.clone();
            self.instance.version = original.version.clone();
        }
    }
    fn reset_mods(&mut self, state: &GruntState) {
        if let Some(original) = state.instances.iter().find(|i| i.id == self.instance.id) {
            self.instance.mods = original.mods.clone();
        }
    }
    fn save(&mut self, state: &mut GruntState) {
        if let Some(instance) = state
            .instances
            .iter_mut()
            .find(|i| i.id == self.instance.id)
        {
            *instance = self.instance.clone();
        }
    }
    fn on_changed(&mut self, state: &GruntState) {
        if let Some(original) = state.instances.iter().find(|i| i.id == self.instance.id) {
            self.changed = self.instance.ne(original);
        }
    }
    fn fetch_current_page_images(&mut self, state: &GruntState) -> ScreenOutput<Message> {
        let mut output = ScreenOutput::none();
        for m in self.instance.mods.iter() {
            if let ModSource::ModDb { mod_id, logo, .. } = &m.source {
                if state.image_cache.peek(mod_id).is_some() {
                    continue;
                } else {
                    if let Some(logo) = &logo
                        && self.requested_images.insert(*mod_id)
                    {
                        output = output.action_add(GruntAction::GetImageLocal {
                            id: *mod_id,
                            path: logo.to_path_buf(),
                        })
                    }
                }
            }
        }
        output
    }
}
