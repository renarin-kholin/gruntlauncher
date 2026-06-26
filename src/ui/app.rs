use iced::{
    Border, Element, Length, Size, Task, Theme, alignment,
    border::Radius,
    padding,
    widget::{column, container, opaque, stack, text},
    window::{icon, settings::PlatformSpecific},
};
use tracing::info;

use crate::{
    assets::GRUNT_ICON,
    core::{config::Config, instance::GruntInstance},
    services::{self, config::LoadConfigError, instance::InstancesError},
    ui::{
        GruntState,
        theme::grunt_theme,
        views::{Screen, add_instance, home},
    },
};
const GRUNT_LAUNCHER_ID: &str = "com.renarin.gruntlauncher";
#[derive(Debug, Clone)]
pub enum GruntMessage {
    ScreenSwitched,

    HomeMessage(home::Message),
    AddInstanceMessage(add_instance::Message),

    InstancesLoaded(Result<Vec<GruntInstance>, InstancesError>),
    InstanceCreated(Result<GruntInstance, InstancesError>),
    ConfigLoaded(Result<Config, LoadConfigError>),
}

#[derive(Clone)]
pub enum GruntAction {
    SwitchScreen(Screen),
    CloseScreen,
    CreateInstance(GruntInstance),
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
            Task::perform(
                async move { crate::services::config::load_config() },
                GruntMessage::ConfigLoaded,
            ),
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

                stack![
                    base,
                    opaque(
                        container(
                            container(column![
                                container(text!("{}", overlay.title()).color(
                                    grunt_theme().extended_palette().secondary.strong.color
                                ))
                                .padding(padding::vertical(5.0).horizontal(15.0))
                                .height(Length::Shrink)
                                .style(|theme: &Theme| container::Style {
                                    background: Some(
                                        theme.extended_palette().background.weaker.color.into()
                                    ),
                                    border: Border {
                                        radius: Radius::default().bottom(0.0).top(4.0),
                                        color: theme.extended_palette().background.weak.color,
                                        width: 1.0,
                                    },
                                    ..container::rounded_box(theme)
                                })
                                .width(Length::Fill),
                                panel
                            ])
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .style(container::bordered_box)
                        )
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .padding(padding::all(40.0))
                        .style(|_theme| container::Style {
                            background: Some(iced::Background::Color(iced::Color {
                                a: 0.8,
                                ..iced::Color::BLACK
                            })),
                            ..Default::default()
                        })
                        .align_x(alignment::Horizontal::Center)
                        .align_y(alignment::Vertical::Center)
                    )
                ]
                .into()
            }
        }
    }
    pub fn update(&mut self, message: GruntMessage) -> Task<GruntMessage> {
        use GruntMessage::*;
        use Screen::*;
        match message {
            ScreenSwitched => {
                if let Some(overlay) = &self.overlay {
                    match overlay {
                        //can send an initial message to an overlay when it opens
                        &Screen::AddInstance(_) => {
                            return Task::done(GruntMessage::AddInstanceMessage(
                                add_instance::Message::ScreenLoaded,
                            ));
                        }
                    }
                }
                Task::none()
            }

            ConfigLoaded(load_result) => {
                if let Ok(config) = load_result {
                    self.state.config = Some(config.clone());
                    return Task::perform(
                        async move {
                            crate::services::instance::load_instances(&config.instances_folder)
                        },
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
            InstanceCreated(instance_result) => {
                if let Ok(instance) = instance_result {
                    self.state.instances.push(instance);
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
            SwitchScreen(s) => {
                self.overlay = Some(s);
                return Task::done(GruntMessage::ScreenSwitched);
            }
            CloseScreen => {
                self.overlay = None;
            }
            CreateInstance(instance) => {
                //TODO: call the instance creation logic on dommain and reload state

                if let Some(config) = self.state.config.clone() {
                    return Task::perform(
                        async move {
                            services::instance::add_instance(instance, &config.instances_folder)
                        },
                        GruntMessage::InstanceCreated,
                    );
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
