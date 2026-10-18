use std::path::PathBuf;

use iced::{
    Element, Size, Subscription, Task,
    alignment::Horizontal,
    event,
    keyboard::{self, key},
    widget::{button, column, image::Handle, operation, progress_bar, row, text},
    window::{icon, settings::PlatformSpecific},
};
use sipper::sipper;
use tracing::{error, info};

use crate::{
    assets::GRUNT_ICON,
    core::{account::AccountStore, config::Config, instance::GruntInstance},
    services::{
        account::{AccountsError, load_session},
        config::ConfigError,
        image::{DecodedImage, ImagesError, load_image, load_image_local},
        instance::InstancesError,
        update::{UpdateStatus, UpdatesError, check_for_update, download_and_apply},
    },
    ui::{
        GruntState,
        app::GruntMessage::{UpdateApplied, UpdateProgress},
        theme::grunt_theme,
        views::{Screen, add_instance, edit_instance, home, settings},
        widget::overlay::overlay_container,
    },
};
const GRUNT_LAUNCHER_ID: &str = "com.renarin.gruntlauncher";
#[derive(Debug, Clone)]
pub enum GruntMessage {
    HomeMessage(home::Message),
    AddInstanceMessage(add_instance::Message),
    EditInstanceMessage(edit_instance::Message),
    SettingsMessage(settings::Message),
    CloseScreen,
    CloseDialog,
    ConfirmDialog,

    InstancesLoaded(Result<Vec<GruntInstance>, InstancesError>),
    ConfigLoaded(Result<Config, ConfigError>),
    ImageLoaded(Result<DecodedImage, ImagesError>, i64),
    SessionLoaded(Result<AccountStore, AccountsError>),
    UpdateChecked(Result<Option<Box<velopack::UpdateInfo>>, UpdatesError>),
    //Only reachable on failure: applying an update successfully exits the process.
    UpdateApplied(Result<(), UpdatesError>),

    UpdateProgress(UpdateStatus),

    IcedEvent(iced::Event),
}
pub enum GruntAction {
    OpenAddInstance,
    OpenSettings,
    OpenEditInstance(GruntInstance),
    OpenDialog,
    CloseScreen,
    CreateInstance(GruntInstance),

    ReloadInstances,

    GetImage { id: i64, url: String },
    GetImageLocal { id: i64, path: PathBuf },
    ApplyUpdate,
}

pub struct GruntLauncher {
    overlay: Option<Screen>,
    update_status: UpdateStatus,
    home: home::Screen,
    state: GruntState,
}

impl GruntLauncher {
    fn theme(&self) -> iced::Theme {
        let choice = match &self.overlay {
            Some(Screen::Settings(screen)) => screen.selected_theme(),
            _ => self.state.config.theme,
        };
        grunt_theme(choice)
    }

    pub fn new() -> (Self, Task<GruntMessage>) {
        (
            Self {
                overlay: None,
                home: home::Screen::new(),
                state: GruntState::default(),
                update_status: UpdateStatus::NotStarted,
            },
            Task::batch([
                Task::perform(
                    crate::services::config::load_config(),
                    GruntMessage::ConfigLoaded,
                ),
                Task::perform(load_session(), GruntMessage::SessionLoaded),
                Task::perform(check_for_update(), GruntMessage::UpdateChecked),
            ]),
        )
    }
    pub fn view_update(&self, percent: i16) -> Element<'_, GruntMessage> {
        let update_column = column![text!("Update in progress: {}%", percent),]
            .spacing(10.0)
            .padding(10.0);
        update_column
            .push(progress_bar(0.0..=100.0, percent as f32))
            .into()
    }
    pub fn view(&self) -> Element<'_, GruntMessage> {
        use GruntMessage::*;
        let base = self.home.view(&self.state).map(HomeMessage);
        if let UpdateStatus::InProgress(percent) = self.update_status {
            return overlay_container(
                base,
                Some(self.view_update(percent)),
                Some("Update in progress".to_string()),
                None,
                false,
            );
        }
        if let Some(dialog) = &self.state.dialog {
            return overlay_container(
                base,
                Some(
                    column![
                        text!("{}", dialog.message).center(),
                        row![
                            button("Cancel").on_press(CloseDialog),
                            button("Confirm").on_press(ConfirmDialog)
                        ]
                        .spacing(10.0)
                    ]
                    .padding(20.0)
                    .spacing(10.0)
                    .align_x(Horizontal::Center)
                    .into(),
                ),
                Some(dialog.title.clone()),
                Some(CloseDialog),
                true,
            );
        }
        match &self.overlay {
            None => base,
            Some(overlay) => {
                let panel = match overlay {
                    Screen::AddInstance(s) => s.view(&self.state).map(AddInstanceMessage),
                    Screen::EditInstance(s) => s.view(&self.state).map(EditInstanceMessage),
                    Screen::Settings(s) => s.view(&self.state).map(SettingsMessage),
                };
                overlay_container(
                    base,
                    Some(panel),
                    Some(overlay.title()),
                    Some(CloseScreen),
                    false,
                )
            }
        }
    }
    pub fn update(&mut self, message: GruntMessage) -> Task<GruntMessage> {
        use GruntMessage::*;
        use Screen::*;
        match message {
            ConfigLoaded(load_result) => {
                if let Ok(config) = load_result {
                    self.state.config = config.clone();
                    return Task::perform(
                        crate::services::instance::load_instances(config.instances_folder),
                        InstancesLoaded,
                    );
                }
            }
            InstancesLoaded(load_result) => {
                info!("Instances loaded.");
                if let Ok(instances) = load_result {
                    self.state.instances = instances;
                }
            }
            ImageLoaded(result, id) => match result {
                Ok(decoded) => {
                    self.state.image_cache.push(
                        id,
                        Handle::from_rgba(decoded.width, decoded.height, decoded.rgba),
                    );
                }
                Err(e) => {
                    error!("Error while loading image: {:?}", e);
                }
            },
            UpdateChecked(result) => match result {
                Ok(Some(update)) => {
                    info!("Update available: {}", update.TargetFullRelease.Version);
                    self.state.available_update = Some(update);
                }
                Ok(None) => info!("No update available."),
                Err(e) => error!("Error while checking for updates: {}", e),
            },
            UpdateApplied(result) => {
                if let Err(e) = result {
                    error!("Error while applying update: {}", e);
                }
            }
            SessionLoaded(load_result) => {
                info!("Session info loaded.");
                match load_result {
                    Ok(account_store) => {
                        self.state.selected_account = account_store.selected_account;
                        self.state.accounts = account_store.accounts;
                    }
                    Err(e) => {
                        error!("Error while loading accounts store: {}", e);
                    }
                }
            }

            //Screen message wrappers
            HomeMessage(m) => {
                let out = self.home.update(m, &mut self.state);
                return Task::batch([self.handle_actions(out.actions), out.task.map(HomeMessage)]);
            }
            AddInstanceMessage(m) => {
                if let Some(AddInstance(s)) = &mut self.overlay {
                    let out = s.update(m, &mut self.state);
                    return Task::batch([
                        self.handle_actions(out.actions),
                        out.task.map(AddInstanceMessage),
                    ]);
                }
            }
            EditInstanceMessage(m) => {
                if let Some(EditInstance(s)) = &mut self.overlay {
                    let out = s.update(m, &mut self.state);
                    return Task::batch([
                        self.handle_actions(out.actions),
                        out.task.map(EditInstanceMessage),
                    ]);
                }
            }
            SettingsMessage(m) => {
                if let Some(Settings(s)) = &mut self.overlay {
                    let out = s.update(m, &mut self.state);
                    return Task::batch([
                        self.handle_actions(out.actions),
                        out.task.map(SettingsMessage),
                    ]);
                }
                // A save can finish after the user has closed the settings overlay.
                match m {
                    settings::Message::ConfigSaved(Ok(config))
                    | settings::Message::ConfigReset(Ok(config)) => {
                        let paths_changed = self.state.config.instances_folder
                            != config.instances_folder
                            || self.state.config.installations_folder
                                != config.installations_folder;
                        self.state.config = config;
                        if paths_changed {
                            return self.handle_action(GruntAction::ReloadInstances);
                        }
                    }
                    settings::Message::ConfigSaved(Err(error))
                    | settings::Message::ConfigReset(Err(error)) => error!("{error}"),
                    _ => {}
                }
            }
            CloseScreen => {
                self.overlay = None;
            }
            CloseDialog => {
                if let Some(dialog) = self.state.dialog.take()
                    && let Some(cancel) = dialog.cancel
                {
                    return Task::done(cancel);
                }
                self.state.dialog = None;
            }
            ConfirmDialog => {
                if let Some(dialog) = self.state.dialog.take()
                    && let Some(confirm) = dialog.confirm
                {
                    return Task::done(confirm);
                }
                self.state.dialog = None;
            }
            UpdateProgress(status) => {
                self.update_status = status;
            }
            IcedEvent(event) => {
                if let iced::Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(key::Named::Tab),
                    modifiers,
                    ..
                }) = event
                {
                    if modifiers.shift() {
                        return operation::focus_previous();
                    } else {
                        return operation::focus_next();
                    }
                }
            }
        }
        Task::none()
    }
    fn handle_action(&mut self, action: GruntAction) -> Task<GruntMessage> {
        use GruntAction::*;
        match action {
            OpenAddInstance => {
                let (screen, task) = add_instance::Screen::new(&mut self.state);
                self.overlay = Some(Screen::AddInstance(Box::new(screen)));
                return task.map(GruntMessage::AddInstanceMessage);
            }
            OpenEditInstance(instance) => {
                let (screen, task) = edit_instance::Screen::new(&mut self.state, instance);
                self.overlay = Some(Screen::EditInstance(Box::new(screen)));
                return task.map(GruntMessage::EditInstanceMessage);
            }
            OpenSettings => {
                let (screen, task) = settings::Screen::new(&mut self.state);
                self.overlay = Some(Screen::Settings(Box::new(screen)));
                return task.map(GruntMessage::SettingsMessage);
            }
            CloseScreen => {
                self.overlay = None;
            }
            CreateInstance(instance) => {
                self.state.instances.push(instance);
            }
            ReloadInstances => {
                return Task::perform(
                    crate::services::instance::load_instances(
                        self.state.config.instances_folder.clone(),
                    ),
                    GruntMessage::InstancesLoaded,
                );
            }
            GetImage { id, url } => {
                return Task::perform(load_image(url), move |bytes| {
                    GruntMessage::ImageLoaded(bytes, id)
                });
            }
            GetImageLocal { id, path } => {
                return Task::perform(load_image_local(path), move |bytes| {
                    GruntMessage::ImageLoaded(bytes, id)
                });
            }
            ApplyUpdate => {
                if let Some(update) = self.state.available_update.clone() {
                    let task = Task::sip(
                        sipper(|mut progress| async move {
                            download_and_apply(update, &mut progress).await
                        }),
                        UpdateProgress,
                        UpdateApplied,
                    );
                    return task;
                }
            }
            OpenDialog => {}
        }
        Task::none()
    }
    fn handle_actions(&mut self, actions: Vec<GruntAction>) -> Task<GruntMessage> {
        Task::batch(
            actions
                .into_iter()
                .map(|a| self.handle_action(a))
                .collect::<Vec<_>>(),
        )
    }
    fn subscription(&self) -> Subscription<GruntMessage> {
        event::listen().map(GruntMessage::IcedEvent)
    }
}

fn settings() -> iced::Settings {
    iced::Settings {
        id: Some(GRUNT_LAUNCHER_ID.to_string()),
        ..Default::default()
    }
}
fn window_settings() -> iced::window::Settings {
    let icon = image::load_from_memory(GRUNT_ICON)
        .expect("Could not load the application icon from memory.");
    iced::window::Settings {
        size: Size::new(1200.0, 700.0),
        platform_specific: PlatformSpecific {
            #[cfg(target_os = "linux")]
            application_id: GRUNT_LAUNCHER_ID.into(),
            ..PlatformSpecific::default()
        },
        icon: icon::from_rgba(icon.to_rgba8().into_raw(), icon.width(), icon.height()).ok(),
        // icon: icon::from_file("assets/icons/logo.png").ok(),
        ..Default::default()
    }
}
pub fn run() -> iced::Result {
    iced::application(
        GruntLauncher::new,
        GruntLauncher::update,
        GruntLauncher::view,
    )
    .settings(settings())
    .window(window_settings())
    .subscription(GruntLauncher::subscription)
    .theme(GruntLauncher::theme)
    .title("Grunt Launcher")
    .run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::ThemeChoice;

    #[test]
    fn theme_preview_discard_and_close_follow_saved_config() {
        let (mut app, _) = GruntLauncher::new();
        let original = app.theme().palette();
        let _ = app.handle_action(GruntAction::OpenSettings);
        let _ = app.update(GruntMessage::SettingsMessage(
            settings::Message::ThemeChanged(ThemeChoice::LightHighContrast),
        ));
        assert_eq!(app.theme().palette().background, iced::Color::WHITE);
        assert_eq!(app.state.config.theme, ThemeChoice::Default);

        let _ = app.update(GruntMessage::SettingsMessage(
            settings::Message::DiscardConfig,
        ));
        assert_eq!(app.theme().palette(), original);
        let _ = app.update(GruntMessage::SettingsMessage(
            settings::Message::ThemeChanged(ThemeChoice::LightHighContrast),
        ));
        let _ = app.update(GruntMessage::CloseScreen);
        assert_eq!(app.theme().palette(), original);
    }

    #[test]
    fn saved_theme_applies_after_closing_settings_and_after_config_reload() {
        let (mut app, _) = GruntLauncher::new();
        let mut saved = app.state.config.clone();
        saved.theme = ThemeChoice::LightHighContrast;
        let _ = app.handle_action(GruntAction::OpenSettings);
        let _ = app.update(GruntMessage::CloseScreen);
        let _ = app.update(GruntMessage::SettingsMessage(
            settings::Message::ConfigSaved(Ok(saved.clone())),
        ));
        assert_eq!(app.state.config.theme, ThemeChoice::LightHighContrast);
        assert_eq!(app.theme().palette().background, iced::Color::WHITE);

        let source = toml::to_string_pretty(&saved).unwrap();
        let (mut restarted, _) = GruntLauncher::new();
        let _ = restarted.update(GruntMessage::ConfigLoaded(Ok(
            toml::from_str(&source).unwrap()
        )));
        assert_eq!(restarted.theme().palette(), app.theme().palette());
    }
}
