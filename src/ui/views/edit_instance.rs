use crate::ui::views::ScreenOutput;
use crate::ui::GruntState;
use iced::widget::space;
use iced::{Element, Task};

pub struct Screen {}

#[derive(Debug, Clone)]
pub enum Message {}

impl Screen {
    pub fn new(state: &GruntState) -> (Self, Task<Message>) {
        (Self {}, Task::none())
    }
    pub fn view<'a>(&'a self, state: &'a GruntState) -> Element<'a, Message> {
        space().into()
    }
    pub fn update(&mut self, message: Message, state: &mut GruntState) -> ScreenOutput<Message> {
        ScreenOutput::none()
    }
}
