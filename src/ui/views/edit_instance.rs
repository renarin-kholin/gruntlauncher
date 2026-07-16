use crate::assets::GRUNT_ICON;
use crate::core::game_mod::{GameMod, ModSource};
use crate::core::instance::GruntInstance;
use crate::core::version::{GameVersion, VersionCatalog};
use crate::services::game_mod::{ModDetail, ModDownloadProgress, ModsError, Release, download_mod};
use crate::services::image::{ImagesError, save_image};
use crate::services::instance::{InstancesError, save_instance};
use crate::services::version::{VersionsError, load_versions};
use crate::ui::component::mod_browser::{self, ModBrowser, Mode};
use crate::ui::views::ScreenOutput;
use crate::ui::widget::overlay::overlay_container;
use crate::ui::widget::table::{self, TableColumn};
use crate::ui::{GruntAction, GruntState};
use futures::stream::{self, StreamExt};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{center, container, progress_bar, right, right_center, space, text_input};
use iced::{
    Element, Length, Task, padding,
    widget::{button, column, image, row, rule, scrollable, text},
};
use iced_aw::spinner;
use sipper::{Sipper, Straw, sipper};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use thiserror::Error;
use tracing::{error, warn};

#[derive(Debug, Clone)]
pub enum Message {
    Navigate(Tab),

    //Form element events
    NameChanged(String),
    SelectVersion(usize),
    DiscardChanges,
    SaveChanges,
    AddMod,
    ChangeModVersion(i64),
    RemoveMod(i64),
    CloseModBrowser,
    ApplyPicks,
    ModBrowserMessage(mod_browser::Message),

    VersionsLoaded(Result<Vec<GameVersion>, VersionsError>),
    SaveProgress((i64, ModDownloadProgress)),
    InstanceSaved(Result<(Tab, GruntInstance), EditError>),
}

#[derive(Clone, Debug, Error)]
pub enum EditError {
    #[error("Error while saving the instance: {0}")]
    Instances(#[from] InstancesError),
    #[error("Error while downloading mods: {0}")]
    Mods(#[from] ModsError),
    #[error("Error while saving the mod logo: {0}")]
    Images(#[from] ImagesError),
}
#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    General,
    Mods,
}
pub struct Screen {
    selected_tab: Tab,
    instance: GruntInstance,
    installing: bool,
    version_columns: Vec<TableColumn>,
    version_rows: Vec<Vec<String>>,
    icon_handle: image::Handle,
    requested_images: HashSet<i64>,
    mod_browser: ModBrowser,
    show_mod_browser: bool,
    pending_picks: Vec<(Box<ModDetail>, Release)>,
    mod_download_progress: HashMap<i64, ModDownloadProgress>,
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
            installing: false,
            version_columns: vec![
                TableColumn::new("Version", 150.0).min_width(80.0),
                TableColumn::new("Type", 300.0).min_width(80.0),
            ],
            version_rows: vec![],
            icon_handle: image::Handle::from_bytes(GRUNT_ICON),
            requested_images: HashSet::new(),
            mod_browser: ModBrowser::new(),
            show_mod_browser: false,
            pending_picks: vec![],
            mod_download_progress: HashMap::new(),
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
    fn original<'a>(&self, state: &'a GruntState) -> Option<&'a GruntInstance> {
        state.instances.iter().find(|i| i.id == self.instance.id)
    }
    fn general_changed(&self, state: &GruntState) -> bool {
        self.original(state)
            .is_some_and(|o| o.name != self.instance.name || o.version != self.instance.version)
    }
    fn mods_changed(&self, state: &GruntState) -> bool {
        !self.pending_picks.is_empty()
            || self
                .original(state)
                .is_some_and(|o| o.mods != self.instance.mods)
    }
    fn pending_for(&self, id: i64) -> Option<&(Box<ModDetail>, Release)> {
        self.pending_picks.iter().find(|(d, _)| d.modid == id)
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
                    button("Discard Changes")
                        .on_press_maybe(self.general_changed(state).then_some(DiscardChanges)),
                    button("Save")
                        .on_press_maybe(self.general_changed(state).then_some(SaveChanges))
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
            let version_label = match self.pending_for(*mod_id) {
                Some((_, release)) => {
                    text!("{} \u{2192} {} (pending save)", version, release.modversion)
                }
                None => text!("{}", version),
            };

            column![
                button(
                    row![
                        container(image(mod_logo).height(50.0).width(50.0))
                            .style(container::bordered_box),
                        column![text!("{}", name), version_label].spacing(5.0),
                        right_center(
                            row![
                                button("Change Version").on_press(ChangeModVersion(*mod_id)),
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
    fn view_pending_item<'a>(
        &'a self,
        detail: &'a ModDetail,
        release: &'a Release,
        state: &'a GruntState,
    ) -> Element<'a, Message> {
        use Message::*;
        let mut mod_logo = self.icon_handle.clone();
        if let Some(logo) = state.image_cache.peek(&detail.modid) {
            mod_logo = logo.clone();
        }
        column![
            button(
                row![
                    container(image(mod_logo).height(50.0).width(50.0))
                        .style(container::bordered_box),
                    column![
                        text!("{}", detail.name),
                        text!("{} (pending save)", release.modversion)
                    ]
                    .spacing(5.0),
                    right_center(
                        row![button("Remove").on_press(RemoveMod(detail.modid))]
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
    fn is_installed_mod(&self, id: i64) -> bool {
        self.instance
            .mods
            .iter()
            .any(|m| matches!(m.source, ModSource::ModDb { mod_id, .. } if mod_id == id))
    }
    pub fn view_mods<'a>(&'a self, state: &'a GruntState) -> Element<'a, Message> {
        use Message::*;
        let installed = self
            .instance
            .mods
            .iter()
            .map(|m| self.view_mod_item(m, state));
        let pending = self
            .pending_picks
            .iter()
            .filter(|(d, _)| !self.is_installed_mod(d.modid))
            .map(|(d, r)| self.view_pending_item(d, r, state));
        column![
            column![text!("Mods")].padding(10.0),
            scrollable(column(installed.chain(pending)))
                .height(Length::Fill)
                .width(Length::Fill),
            space().height(10.0),
            row![
                button("Add Mod").on_press(AddMod),
                right(
                    row![
                        button("Discard Changes").on_press_maybe(
                            (self.mods_changed(state) && !self.installing)
                                .then_some(DiscardChanges)
                        ),
                        button("Save").on_press_maybe(
                            (self.mods_changed(state) && !self.installing).then_some(SaveChanges)
                        )
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
    pub fn view_add_mods<'a>(&'a self, state: &'a GruntState) -> Element<'a, Message> {
        use Message::*;
        column![
            self.mod_browser
                .view(Some(self.instance.version.clone()), state)
                .map(ModBrowserMessage),
            rule::horizontal(1.0),
            right(self.overlay_buttons())
        ]
        .spacing(10.0)
        .into()
    }
    fn overlay_buttons(&self) -> Element<'_, Message> {
        use Message::*;
        row![
            button("Cancel").on_press(CloseModBrowser),
            button("Apply").on_press_maybe(self.maybe_changed(ApplyPicks))
        ]
        .spacing(10.0)
        .padding(10.0)
        .into()
    }
    fn maybe_changed<Message>(&self, m: Message) -> Option<Message> {
        use Mode;
        match self.mod_browser.mode {
            Mode::Detail {
                installed_release_id,
            } => {
                if self.mod_browser.mod_version_changed(installed_release_id) {
                    Some(m)
                } else {
                    None
                }
            }
            Mode::Browsing => {
                if self.mod_browser.mod_list_changed(&self.instance.mods) {
                    Some(m)
                } else {
                    None
                }
            }
        }
    }
    pub fn view_change_version<'a>(&'a self, state: &'a GruntState) -> Element<'a, Message> {
        use Message::*;
        column![
            self.mod_browser
                .view(Some(self.instance.version.clone()), state)
                .map(ModBrowserMessage),
            rule::horizontal(1.0),
            right(self.overlay_buttons())
        ]
        .spacing(10.0)
        .into()
    }
    fn view_save_progress(&self) -> Element<'_, Message> {
        let mut progress_column = column![text!("Downloading Mods...")]
            .spacing(10.0)
            .padding(10.0);
        for (modid, mp) in self.mod_download_progress.iter() {
            use ModDownloadProgress::*;
            let mod_name = self
                .pending_for(*modid)
                .map(|(d, _)| d.name.clone())
                .unwrap_or_else(|| "Unknown Mod".to_string());
            progress_column = match mp {
                Queued => progress_column.push(text!("{} queued for download.", mod_name)),
                Downloading { downloaded, total } => progress_column.push(column![
                    text!("Downloading {}", mod_name),
                    progress_bar(0.0..=(*total as f32), *downloaded as f32)
                ]),
                Downloaded => progress_column.push(text!("Downloaded {}", mod_name)),
                Failed(err) => {
                    progress_column.push(text!("Failed to download {}: {:?}", mod_name, err))
                }
            };
        }
        progress_column.into()
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
        let (children, title) = if self.installing {
            (
                Some(self.view_save_progress()),
                Some("Saving instance".to_string()),
            )
        } else if self.show_mod_browser {
            use mod_browser::Mode::*;
            match self.mod_browser.mode {
                Detail { .. } => (
                    Some(self.view_change_version(state)),
                    Some("Change mod version".to_string()),
                ),
                Browsing => (
                    Some(self.view_add_mods(state)),
                    Some("Add new mods".to_string()),
                ),
            }
        } else {
            (None, None)
        };
        overlay_container(
            base.into(),
            children,
            title,
            (!self.installing).then_some(CloseModBrowser),
        )
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
            }
            SelectVersion(i) => {
                if let VersionCatalog::Loaded { versions } = &state.vs_versions
                    && let Some(version) = versions.get(i).cloned()
                {
                    self.instance.version = version;
                }
            }
            RemoveMod(id) => {
                self.pending_picks.retain(|(d, _)| d.modid != id);
                self.instance.mods.retain(|m| {
                    if let ModSource::ModDb { mod_id, .. } = m.source {
                        mod_id != id
                    } else {
                        //TODO: support working with local mods later
                        true
                    }
                });
            }
            DiscardChanges => match self.selected_tab {
                Tab::General => self.reset_general(state),
                Tab::Mods => self.reset_mods(state),
            },
            SaveChanges => {
                if self.installing {
                    return ScreenOutput::none();
                }
                let Some(original) = self.original(state) else {
                    return ScreenOutput::none();
                };
                let mut merged = original.clone();
                let tab = self.selected_tab.clone();
                let original_mods = original.mods.clone();
                let pending = match tab {
                    Tab::General => {
                        merged.name = self.instance.name.clone();
                        merged.version = self.instance.version.clone();
                        vec![]
                    }
                    Tab::Mods => {
                        merged.mods = self.instance.mods.clone();
                        self.pending_picks.clone()
                    }
                };
                self.installing = !pending.is_empty();
                self.mod_download_progress.clear();
                let instance_dir = state
                    .config
                    .instances_folder
                    .join(self.instance.id.to_string());
                let instances_folder = state.config.instances_folder.clone();
                return ScreenOutput::task(Task::sip(
                    Self::save_changes(
                        tab,
                        merged,
                        original_mods,
                        pending,
                        instance_dir,
                        instances_folder,
                    ),
                    SaveProgress,
                    InstanceSaved,
                ));
            }
            SaveProgress((modid, modprogress)) => {
                self.mod_download_progress
                    .entry(modid)
                    .and_modify(|m| *m = modprogress.clone())
                    .or_insert(modprogress);
            }
            InstanceSaved(result) => {
                self.installing = false;
                match result {
                    Ok((tab, saved)) => {
                        if tab == Tab::Mods {
                            for (detail, _) in self.pending_picks.drain(..) {
                                state.image_cache.pop(&detail.modid);
                                self.requested_images.remove(&detail.modid);
                            }
                            self.instance.mods = saved.mods.clone();
                        }
                        // Sync the in-memory original only after the disk write succeeds.
                        if let Some(entry) = state.instances.iter_mut().find(|i| i.id == saved.id) {
                            *entry = saved;
                        }
                        return self.fetch_current_page_images(state);
                    }
                    Err(e) => {
                        error!("{e}");
                    }
                }
            }
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

            AddMod => {
                self.mod_browser = ModBrowser::new();
                self.show_mod_browser = true;
                self.mod_browser.mode = mod_browser::Mode::Browsing;
                self.mod_browser.installed_mods = self.instance.mods.clone();
            }
            ChangeModVersion(id) => {
                self.mod_browser = ModBrowser::new();
                self.show_mod_browser = true;
                let Some(game_mod) = self.instance.mods.iter().find(move |m| {
                    if let ModSource::ModDb { mod_id, .. } = m.source {
                        mod_id == id
                    } else {
                        false
                    }
                }) else {
                    error!("Couldn't find a mod with that id");
                    return ScreenOutput::none();
                };
                let ModSource::ModDb { release_id, .. } = game_mod.source else {
                    unreachable!("Invalid mod type");
                };

                self.mod_browser.mode = mod_browser::Mode::Detail {
                    installed_release_id: release_id,
                };
                self.mod_browser.installed_mods = self.instance.mods.clone();
                return self
                    .mod_browser
                    .update(
                        mod_browser::Message::SelectMod(id),
                        &mut Some(self.instance.version.clone()),
                        state,
                    )
                    .map(ModBrowserMessage);
            }
            ApplyPicks => {
                for (detail, release) in self.mod_browser.picks() {
                    let already_installed = self.instance.mods.iter().any(|m| {
                        matches!(
                            m.source,
                            ModSource::ModDb { mod_id, release_id, .. }
                                if mod_id == detail.modid && release_id == release.releaseid
                        )
                    });
                    if already_installed {
                        self.pending_picks.retain(|(d, _)| d.modid != detail.modid);
                        continue;
                    }
                    if let Some(existing) = self
                        .pending_picks
                        .iter_mut()
                        .find(|(d, _)| d.modid == detail.modid)
                    {
                        *existing = (detail, release);
                    } else {
                        self.pending_picks.push((detail, release));
                    }
                }
                self.show_mod_browser = false;
            }
            CloseModBrowser => {
                self.show_mod_browser = false;
            }
            ModBrowserMessage(m) => {
                let mut version = Some(self.instance.version.clone());
                return self
                    .mod_browser
                    .update(m, &mut version, state)
                    .map(ModBrowserMessage);
            }
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
        // Pending picks were never downloaded, so discarding is pure memory.
        self.pending_picks.clear();
        if let Some(original) = state.instances.iter().find(|i| i.id == self.instance.id) {
            self.instance.mods = original.mods.clone();
        }
    }
    fn save_changes(
        tab: Tab,
        mut merged: GruntInstance,
        original_mods: Vec<GameMod>,
        pending: Vec<(Box<ModDetail>, Release)>,
        instance_dir: PathBuf,
        instances_folder: PathBuf,
    ) -> impl Straw<(Tab, GruntInstance), (i64, ModDownloadProgress), EditError> {
        sipper(async move |progress| {
            let mod_folder = instance_dir.join("Mods");
            let logo_folder = instance_dir.join("Logos");
            let results: Vec<Result<GameMod, EditError>> = stream::iter(pending)
                .map(|(detail, release): (Box<ModDetail>, Release)| {
                    let modid = detail.modid;
                    let releaseid = release.releaseid;
                    let mod_folder = mod_folder.clone();
                    let logo_folder = logo_folder.clone();
                    let progress = progress.clone();
                    async move {
                        let modversion = release.modversion.clone();
                        let file = sipper(async move |mut mod_progress| {
                            download_mod(mod_folder, release, &mut mod_progress).await
                        })
                        .with(move |p| (modid, p))
                        .run(&progress)
                        .await?;
                        let logo = if let Some(logo_file) = detail.logofile.clone() {
                            Some(
                                save_image(logo_folder.join(format!("{modid}.png")), logo_file)
                                    .await?,
                            )
                        } else {
                            None
                        };
                        Ok::<GameMod, EditError>(GameMod::moddb(
                            modid,
                            releaseid,
                            file,
                            logo,
                            detail.name.clone(),
                            detail.text.clone(),
                            modversion,
                        ))
                    }
                })
                .buffer_unordered(3)
                .collect()
                .await;
            if let Some(err) = results.iter().find_map(|r| r.as_ref().err()) {
                let err = err.clone();
                for game_mod in results.into_iter().flatten() {
                    if let Err(e) = tokio::fs::remove_file(&game_mod.file).await {
                        warn!(
                            "Could not clean up partially downloaded mod {:?}: {e}",
                            game_mod.file
                        );
                    }
                }
                return Err(err);
            }
            for game_mod in results.into_iter().flatten() {
                let ModSource::ModDb { mod_id: modid, .. } = game_mod.source else {
                    continue;
                };
                if let Some(existing) = merged.mods.iter_mut().find(
                    |m| matches!(m.source, ModSource::ModDb { mod_id, .. } if mod_id == modid),
                ) {
                    *existing = game_mod;
                } else {
                    merged.mods.push(game_mod);
                }
            }
            for file in Self::obsolete_mod_files(&original_mods, &merged.mods) {
                if let Err(e) = tokio::fs::remove_file(&file).await {
                    warn!("Could not remove obsolete mod file {file:?}: {e}");
                }
            }
            save_instance(merged.clone(), instances_folder).await?;
            Ok((tab, merged))
        })
    }
    fn obsolete_mod_files(original: &[GameMod], saved: &[GameMod]) -> Vec<PathBuf> {
        let mut obsolete = vec![];
        for m in original {
            let ModSource::ModDb {
                mod_id,
                release_id,
                logo,
                ..
            } = &m.source
            else {
                continue;
            };
            let same_release = saved.iter().any(|s| {
                matches!(
                    s.source,
                    ModSource::ModDb { mod_id: id, release_id: rid, .. }
                        if id == *mod_id && rid == *release_id
                )
            });
            let file_still_used = saved.iter().any(|s| s.file == m.file);
            if !same_release && !file_still_used {
                obsolete.push(m.file.clone());
            }
            let mod_still_present = saved
                .iter()
                .any(|s| matches!(s.source, ModSource::ModDb { mod_id: id, .. } if id == *mod_id));
            if !mod_still_present && let Some(logo) = logo {
                obsolete.push(logo.clone());
            }
        }
        obsolete
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
