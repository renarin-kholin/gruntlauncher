use iced::{
    padding,
    widget::{button, column, row, rule, scrollable, space},
    Element, Length, Task,
};

use crate::ui::{views::ScreenOutput, GruntState};

#[derive(Clone, Debug, Default, PartialEq)]
enum Tab {
    #[default]
    General,
    About,
}
pub struct Screen {
    selected_tab: Tab,
}

#[derive(Clone, Debug)]
pub enum Message {
    Navigate(Tab),
}

impl Screen {
    pub fn new(_state: &mut GruntState) -> (Self, Task<Message>) {
        (
            Self {
                selected_tab: Tab::default(),
            },
            Task::none(),
        )
    }
    pub fn view_general<'a>(&'a self, state: &'a GruntState) -> Element<'a, Message> {
        space().into()
    }
    pub fn view_about<'a>(&'a self, state: &'a GruntState) -> Element<'a, Message> {
        space().into()
    }
    pub fn view<'a>(&'a self, state: &'a GruntState) -> Element<'a, Message> {
        use Message::*;
        row![
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
                    button("About")
                        .on_press(Navigate(Tab::About))
                        .width(Length::Fill)
                        .style(|theme, mut status| {
                            if self.selected_tab == Tab::About {
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
                Tab::About => self.view_about(state),
            }
        ]
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    }
    pub fn update(&mut self, message: Message, state: &mut GruntState) -> ScreenOutput<Message> {
        use Message::*;
        match message {
            Navigate(tab) => {
                self.selected_tab = tab;
            }
        }
        ScreenOutput::none()
    }
}
