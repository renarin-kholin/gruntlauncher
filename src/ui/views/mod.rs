use iced::Task;

use crate::ui::GruntAction;
pub mod add_instance;
pub mod home;
//The current actively shown Screen
#[derive(Clone)]
pub enum Screen {
    AddInstance(add_instance::Screen),
}

impl Screen {
    pub fn title(&self) -> String {
        match self {
            Screen::AddInstance(_) => "Add a new Instance".to_string(),
        }
    }
}

pub struct ScreenOutput<MessageT> {
    pub task: Task<MessageT>,
    pub actions: Vec<GruntAction>,
}

impl<MessageT> ScreenOutput<MessageT> {
    pub fn none() -> Self {
        Self {
            actions: vec![],
            task: Task::none(),
        }
    }
    pub fn action(a: GruntAction) -> Self {
        Self {
            actions: vec![a],
            task: Task::none(),
        }
    }
    pub fn action_add(mut self, a: GruntAction) -> Self {
        self.actions.push(a);
        self
    }
    pub fn task(t: Task<MessageT>) -> Self {
        Self {
            actions: vec![],
            task: t,
        }
    }
}
