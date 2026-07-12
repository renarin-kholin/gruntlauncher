use iced::{
    Element, Size, Task,
    widget::image::Handle,
    window::{icon, settings::PlatformSpecific},
};
use tracing::{error, info};

use crate::{
    assets::GRUNT_ICON,
    core::{account::AccountStore, config::Config, instance::GruntInstance},
    services::{
        account::{AccountsError, load_session},
        config::LoadConfigError,
        image::{DecodedImage, ImagesError, load_image},
        instance::InstancesError,
        update::{UpdatesError, check_for_update, download_and_apply},
    },
    ui::{
        GruntState,
        theme::grunt_theme,
        views::{Screen, add_instance, home},
        widget::overlay::overlay_container,
    },
};
const GRUNT_LAUNCHER_ID: &str = "com.renarin.gruntlauncher";
#[derive(Debug, Clone)]
pub enum GruntMessage {
    HomeMessage(home::Message),
    AddInstanceMessage(add_instance::Message),

    InstancesLoaded(Result<Vec<GruntInstance>, InstancesError>),
    ConfigLoaded(Result<Config, LoadConfigError>),
    ImageLoaded(Result<DecodedImage, ImagesError>, i64),
    SessionLoaded(Result<AccountStore, AccountsError>),
    UpdateChecked(Result<Option<Box<velopack::UpdateInfo>>, UpdatesError>),
    //Only reachable on failure: applying an update successfully exits the process.
    UpdateApplied(Result<(), UpdatesError>),
}

pub enum GruntAction {
    OpenAddInstance,
    CloseScreen,
    CreateInstance(GruntInstance),

    GetImage { id: i64, url: String },
    ApplyUpdate,
}

pub struct GruntLauncher {
    overlay: Option<Screen>,
    home: home::Screen,
    state: GruntState,
}

impl GruntLauncher {
    pub fn new() -> (Self, Task<GruntMessage>) {
        (
            Self {
                overlay: None,
                home: home::Screen::new(),
                state: GruntState::default(),
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
    pub fn view(&self) -> Element<'_, GruntMessage> {
        use GruntMessage::*;
        let base = self.home.view(&self.state).map(HomeMessage);
        match &self.overlay {
            None => base,
            Some(overlay) => {
                let panel = match overlay {
                    Screen::AddInstance(s) => s.view(&self.state).map(AddInstanceMessage),
                };
                overlay_container(base, Some(panel), Some(overlay.title()))
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
                Task::none()
            }
            InstancesLoaded(load_result) => {
                info!("Instances loaded.");
                if let Ok(instances) = load_result {
                    self.state.instances.extend(instances);
                }
                Task::none()
            }
            ImageLoaded(result, id) => {
                match result {
                    Ok(decoded) => {
                        self.state.image_cache.push(
                            id,
                            Handle::from_rgba(decoded.width, decoded.height, decoded.rgba),
                        );
                    }
                    Err(e) => {
                        error!("Error while loading image: {:?}", e);
                    }
                }
                Task::none()
            }
            UpdateChecked(result) => {
                match result {
                    Ok(Some(update)) => {
                        info!("Update available: {}", update.TargetFullRelease.Version);
                        self.state.available_update = Some(update);
                    }
                    Ok(None) => info!("No update available."),
                    Err(e) => error!("Error while checking for updates: {}", e),
                }
                Task::none()
            }
            UpdateApplied(result) => {
                if let Err(e) = result {
                    error!("Error while applying update: {}", e);
                }
                Task::none()
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
                Task::none()
            }

            //Screen message wrappers
            HomeMessage(m) => {
                let out = self.home.update(m, &mut self.state);
                Task::batch([self.handle_actions(out.actions), out.task.map(HomeMessage)])
            }
            AddInstanceMessage(m) if let Some(AddInstance(s)) = &mut self.overlay => {
                let out = s.update(m, &mut self.state);
                Task::batch([
                    self.handle_actions(out.actions),
                    out.task.map(AddInstanceMessage),
                ])
            }
            _ => Task::none(),
        }
    }
    fn handle_action(&mut self, action: GruntAction) -> Task<GruntMessage> {
        use GruntAction::*;
        match action {
            OpenAddInstance => {
                let (screen, task) = add_instance::Screen::new(&mut self.state);
                self.overlay = Some(Screen::AddInstance(screen));
                return task.map(GruntMessage::AddInstanceMessage);
            }
            CloseScreen => {
                self.overlay = None;
            }
            CreateInstance(instance) => {
                self.state.instances.push(instance);
            }
            GetImage { id, url } => {
                return Task::perform(load_image(url), move |bytes| {
                    GruntMessage::ImageLoaded(bytes, id)
                });
            }
            ApplyUpdate => {
                if let Some(update) = self.state.available_update.clone() {
                    return Task::perform(download_and_apply(update), GruntMessage::UpdateApplied);
                }
            }
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
    .theme(grunt_theme())
    .title("Grunt Launcher")
    .run()
}
