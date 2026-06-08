use iced::{
    Element, Length, Padding,
    alignment::{Horizontal, Vertical},
    padding,
    widget::{button, column, container, image, row, rule, scrollable, space, text, text_input},
};

use crate::{
    core::instance::GruntInstance,
    ui::{
        GruntAction, GruntState,
        views::ScreenOutput,
        widget::table::{self, TableColumn},
    },
};

#[derive(Clone, Debug, PartialEq, Eq)]
enum VersionType {
    Release,
    PreRelease,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Version {
    pub name: String,
    version_type: VersionType,
    pub released_date: String,
}
impl Version {
    pub fn version_type(&self) -> String {
        match self.version_type {
            VersionType::Release => "Release".to_string(),
            VersionType::PreRelease => "Pre-release".to_string(),
        }
    }
}
#[derive(Clone)]
pub struct Screen {
    instance: GruntInstance,
    columns: Vec<TableColumn>,
    rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    Cancel,
}

impl Screen {
    pub fn new() -> Self {
        let versions = vec![
            Version {
                name: "1.21.2".to_string(),
                version_type: VersionType::Release,
                released_date: "21/02/2026".to_string(),
            };
            20
        ];
        Self {
            instance: GruntInstance {
                name: String::from(""),
            },

            columns: vec![
                TableColumn::new("Version", 150.0).min_width(80.0),
                TableColumn::new("Type", 300.0).min_width(80.0),
            ],
            rows: versions
                .iter()
                .map(|v| vec![v.name.clone(), v.version_type()])
                .collect(),
        }
    }

    pub fn view(&self, _state: &GruntState) -> Element<'_, Message> {
        column![
            //Instance details (name and icon)
            row![
                button(image("assets/icons/logo.png").height(50.0).width(50.0)).style(button::text),
                row![
                    text!("Name: "),
                    text_input("Default name", &self.instance.name).on_input(Message::NameChanged)
                ]
                .align_y(Vertical::Center)
                .spacing(4.0)
            ]
            .align_y(Vertical::Center)
            .spacing(10.0)
            .padding(padding::all(10.0))
            .height(Length::Shrink),
            rule::horizontal(1.0),
            row![
                scrollable(column![
                    button("Basics").width(Length::Fill).style(button::text),
                    button("Mods").width(Length::Fill).style(button::text),
                    button("Review").width(Length::Fill).style(button::text)
                ])
                .width(Length::Fixed(150.0)),
                rule::vertical(1.0),
                column![
                    scrollable(
                        container(table::Table::new(&self.columns, &self.rows).row_height(30.0))
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .padding(padding::all(1.0))
                            .style(container::bordered_box)
                    )
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .style(|theme, status| {
                        scrollable::Style {
                            container: container::bordered_box(theme),
                            ..scrollable::default(theme, status)
                        }
                    }),
                    row![
                        button("Next"),
                        button("Cancel")
                            .on_press(Message::Cancel)
                            .style(button::subtle)
                    ]
                    .spacing(10.0)
                    .align_y(Vertical::Center)
                ]
                .spacing(10.0)
                .padding(padding::all(10.0))
                .align_x(Horizontal::Right)
            ]
            .height(Length::Fill)
            .width(Length::Fill)
        ]
        .into()
    }

    pub fn update(&mut self, message: Message) -> ScreenOutput<Message> {
        use GruntAction::*;
        match message {
            Message::Cancel => ScreenOutput::action(CloseScreen),
            Message::NameChanged(name) => {
                self.instance.name = name;
                ScreenOutput::none()
            }
        }
    }
}
