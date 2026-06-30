use iced::{
    Element, Length, Task,
    alignment::{Horizontal, Vertical},
    padding,
    widget::{self, button, column, image::Handle, row, rule, scrollable, text},
};
use tracing::debug;

use crate::{
    assets::GRUNT_ICON,
    core::instance::{GruntInstance, InstanceId},
    services::instance::{self, InstancesError},
    ui::{GruntAction, GruntState, views::ScreenOutput},
};

pub struct Screen {
    selected_instance: Option<InstanceId>,
    icon_handles: Vec<Handle>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectInstance(InstanceId),
    LaunchInstance,
    InstanceLaunched(Result<(), InstancesError>),
    AddInstance,
}

impl Default for Screen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen {
    pub fn new() -> Self {
        Self {
            selected_instance: None,
            icon_handles: vec![Handle::from_bytes(GRUNT_ICON)],
        }
    }

    fn instance<'a>(&'a self, instance: &'a GruntInstance) -> Element<'a, Message> {
        button(
            column![
                widget::image(self.icon_handles[0].clone())
                    .height(80.0)
                    .width(80.0),
                text!("{}", instance.name)
                    .wrapping(text::Wrapping::Glyph)
                    .center()
            ]
            .height(Length::Fixed(120.0))
            .width(Length::Fixed(100.0))
            .align_x(Horizontal::Center)
            .spacing(10.0),
        )
        .on_press(Message::SelectInstance(instance.id))
        .style(move |theme, status| {
            let mut status = status;
            if let Some(selected_instance) = &self.selected_instance
                && *selected_instance == instance.id
            {
                status = button::Status::Pressed;
            }
            button::Style {
                ..button::subtle(theme, status)
            }
        })
        .into()
    }
    pub fn view<'a>(&'a self, state: &'a GruntState) -> Element<'a, Message> {
        column![
            //Topbar
            row![
                button("Add Instance").on_press(Message::AddInstance),
                rule::vertical(2),
                button("Settings")
            ]
            .padding(padding::all(10.0))
            .spacing(10.0)
            .height(Length::Shrink)
            .align_y(Vertical::Center),
            rule::horizontal(1.0),
            row![
                //Instances
                scrollable(
                    row(state
                        .instances
                        .iter()
                        .map(|instance| self.instance(instance)))
                    .spacing(10.0)
                    .padding(padding::all(10.0))
                    .width(Length::Fill)
                    .wrap()
                )
                .width(Length::FillPortion(5)),
                rule::vertical(1.0),
                //Sidebar
                column![
                    text("Selected instance"),
                    button("Launch")
                        .padding(padding::horizontal(30.0).vertical(7.0))
                        .style(move |theme, mut status| {
                            if self.selected_instance.is_none() {
                                status = button::Status::Disabled;
                            }
                            button::primary(theme, status)
                        })
                        .on_press(Message::LaunchInstance)
                ]
                .align_x(Horizontal::Center)
                .width(Length::FillPortion(1))
                .spacing(10.0)
                .padding(padding::all(10.0))
            ]
        ]
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    }
    pub fn update(&mut self, message: Message, state: &mut GruntState) -> ScreenOutput<Message> {
        use GruntAction::*;
        use Message::*;
        match message {
            AddInstance => {
                return ScreenOutput::action(OpenAddInstance);
            }
            SelectInstance(id) => {
                self.selected_instance = Some(id);
            }
            LaunchInstance => {
                if let (Some(selected_instance), Some(config)) =
                    (self.selected_instance, state.config.clone())
                {
                    let instance = state
                        .instances
                        .iter()
                        .find(|i| i.id == selected_instance)
                        .expect("Could not select instance");
                    return ScreenOutput::task(Task::perform(
                        instance::launch_instance(instance.clone(), config.instances_folder),
                        InstanceLaunched,
                    ));
                }
            }
            InstanceLaunched(result) => {
                debug!("{result:?}");
            }
        }
        ScreenOutput::none()
    }
}
