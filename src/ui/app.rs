use std::{fs::read_to_string, time::Duration};

use iced::{
    Element, Length, Size, Subscription, Task, alignment, padding, time,
    widget::{container, opaque, space, stack, text},
    window::{icon, settings::PlatformSpecific},
};
use iced_webview::{PageType, WebView};

use crate::{
    assets::GRUNT_ICON,
    ui::{
        GruntState,
        theme::grunt_theme,
        views::{Screen, add_instance, home},
    },
};

const GRUNT_LAUNCHER_ID: &str = "com.renarin.gruntlauncher";
#[derive(Debug, Clone)]
pub enum GruntMessage {
    HomeMessage(home::Message),
    AddInstanceMessage(add_instance::Message),

    //WebView Events
    WebViewCreated,
    WebView(iced_webview::Action),
}

#[derive(Clone)]
pub enum GruntAction {
    SwitchScreen(Screen),
    CloseScreen,
}

type Engine = iced_webview::Blitz;
pub struct GruntLauncher {
    overlay: Option<Screen>,
    home: home::Screen,
    state: GruntState,
    webview: Option<WebView<Engine, GruntMessage>>,
    webview_ready: bool,
}

impl GruntLauncher {
    pub fn new() -> (Self, Task<GruntMessage>) {
        let webview = WebView::new()
            .on_create_view(GruntMessage::WebViewCreated)
            .on_action(GruntMessage::WebView);
        (
            Self {
                overlay: None,
                home: home::Screen::new(),
                state: GruntState::default(),
                webview: Some(webview),
                webview_ready: false,
            },
            Task::done(GruntMessage::WebView(iced_webview::Action::CreateView(
                PageType::Html(
                    read_to_string("src/ui/test.html").expect("Could not read htmlfile"),
                ),
            ))),
        )
    }
    pub fn view(&self) -> Element<'_, GruntMessage> {
        use GruntMessage::*;
        let base = self.home.view(&self.state).map(HomeMessage);
        match &self.overlay {
            None => base,
            Some(overlay) => {
                let panel = match overlay {
                    Screen::AddInstance(s) => s
                        .view(
                            &self.state,
                            if self.webview_ready {
                                self.webview.as_ref().map(|webview| {
                                    webview.view().map(add_instance::Message::WebView)
                                })
                            } else {
                                None
                            },
                        )
                        .map(AddInstanceMessage),
                };
                stack![
                    base,
                    opaque(
                        container(
                            container(panel)
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
            HomeMessage(m) => {
                let out = self.home.update(m);
                self.handle_actions(out.actions)
                    .chain(out.task.map(HomeMessage))
            }
            AddInstanceMessage(m) if let Some(AddInstance(s)) = &mut self.overlay => {
                let mut wvtask = Task::none();
                if let add_instance::Message::WebView(a) = &m
                    && let Some(w) = &mut self.webview
                {
                    wvtask = w.update(a.clone());
                }
                let out = s.update(m);
                Task::batch(vec![
                    wvtask,
                    self.handle_actions(out.actions),
                    out.task.map(AddInstanceMessage),
                ])
            }
            WebViewCreated => {
                self.webview_ready = true;
                if let Some(webview) = &mut self.webview {
                    webview.update(iced_webview::Action::ChangeView(0))
                } else {
                    Task::none()
                }
            }
            WebView(action) if let Some(webview) = &mut self.webview => webview.update(action),
            _ => Task::none(),
        }
    }
    fn handle_action(&mut self, action: GruntAction) -> Task<GruntMessage> {
        use GruntAction::*;
        match action {
            SwitchScreen(s) => {
                self.overlay = Some(s);
            }
            CloseScreen => {
                self.overlay = None;
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
    fn subscription(&self) -> Subscription<GruntMessage> {
        time::every(Duration::from_millis(10))
            .map(|_| iced_webview::Action::Update)
            .map(GruntMessage::WebView)
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
    .subscription(GruntLauncher::subscription)
    .settings(settings())
    .window(window_settings())
    .theme(grunt_theme())
    .title("Grunt Launcher")
    .run()
}
