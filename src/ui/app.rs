use iced::{
    Element, Size, Task,
    widget::space,
    window::{Icon, icon, settings::PlatformSpecific},
};

use crate::{
    assets::GRUNT_ICON,
    core::instance::GruntInstance,
    ui::{
        GruntState,
        theme::grunt_theme,
        views::{Screen, add_instance, home},
    },
};

const GRUNT_LAUNCHER_ID: &str = "com.renarin.gruntlauncher";
#[derive(Debug)]
pub enum GruntMessage {
    HomeMessage(home::Message),
    AddInstanceMessage(add_instance::Message),
}
#[derive(Clone)]
pub enum GruntAction {
    SwitchScreen(Screen),
    CloseScreen,
}

pub struct GruntLauncher {
    screen: Screen,
    state: GruntState,
}

impl GruntLauncher {
    pub fn new() -> Self {
        Self {
            screen: Screen::Home(home::Screen::new()),
            state: GruntState::default(),
        }
    }
    pub fn view(&self) -> Element<'_, GruntMessage> {
        use GruntMessage::*;
        match &self.screen {
            Screen::Home(s) => s.view(&self.state).map(HomeMessage),
            Screen::AddInstance(s) => s.view(&self.state).map(AddInstanceMessage),
        }
    }
    pub fn update(&mut self, message: GruntMessage) -> Task<GruntMessage> {
        use GruntMessage::*;
        match message {
            HomeMessage(m) if let Screen::Home(s) = &mut self.screen => {
                let out = s.update(m);
                self.handle_actions(out.actions)
                    .chain(out.task.map(HomeMessage))
            }
            AddInstanceMessage(m) if let Screen::AddInstance(s) = &mut self.screen => {
                let out = s.update(m);
                self.handle_actions(out.actions)
                    .chain(out.task.map(AddInstanceMessage))
            }
            _ => Task::none(),
        }
    }
    fn handle_action(&mut self, action: GruntAction) -> Task<GruntMessage> {
        use GruntAction::*;
        match action {
            SwitchScreen(s) => {
                self.screen = s;
            }
            CloseScreen => {
                self.screen = Screen::Home(home::Screen::new());
            }
        }
        Task::none()
    }
    fn handle_actions(&mut self, actions: Vec<GruntAction>) -> Task<GruntMessage> {
        let tasks = actions
            .iter()
            .map(move |a| self.handle_action(a.clone()))
            .collect::<Vec<_>>();

        Task::batch(tasks)
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
