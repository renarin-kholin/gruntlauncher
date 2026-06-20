use iced::{
    Element, Length, Task,
    alignment::{Horizontal, Vertical},
    padding,
    widget::{
        button, column, container, image, right_center, row, rule, scrollable, text, text_input,
    },
};
use iced_blitzview::web_view;
use uuid::Uuid;

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
    #[expect(dead_code)]
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    Basic,
    Mod,
    Review,
}
impl Step {
    pub fn next(&mut self) {
        match self {
            Self::Basic => *self = Self::Mod,
            Self::Mod => *self = Self::Review,
            Self::Review => *self = Self::Basic,
        }
    }
    pub fn back(&mut self) {
        match self {
            Self::Basic => *self = Self::Review,
            Self::Mod => *self = Self::Basic,
            Self::Review => *self = Self::Mod,
        }
    }
}
#[derive(Clone)]
pub struct Screen {
    instance: GruntInstance,
    columns: Vec<TableColumn>,
    rows: Vec<Vec<String>>,
    step: Step,
    selected_mod: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    SelectMod(usize),
    Navigate(Step),
    OpenInBrowser(String),
    Next,
    Back,
    Cancel,
    CreateInstance,

    //Webview events
    ModViewPageFetched(Result<String, String>),
}

impl Default for Screen {
    fn default() -> Self {
        Self::new()
    }
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
                id: Uuid::new_v4(),
                mods: vec![],
            },

            columns: vec![
                TableColumn::new("Version", 150.0).min_width(80.0),
                TableColumn::new("Type", 300.0).min_width(80.0),
            ],
            rows: versions
                .iter()
                .map(|v| vec![v.name.clone(), v.version_type()])
                .collect(),
            step: Step::Basic,
            selected_mod: None,
        }
    }

    fn view_basic(&self, _state: &GruntState) -> Element<'_, Message> {
        use Message::*;
        column![
            //Instance details (name and icon)
            row![
                button(image("assets/icons/logo.png").height(50.0).width(50.0)).style(button::text),
                column![
                    text!("Instance Name "),
                    text_input("Default name", &self.instance.name).on_input(NameChanged)
                ]
                .spacing(5.0)
            ]
            .align_y(Vertical::Center)
            .spacing(10.0)
            .padding(padding::all(10.0))
            .height(Length::Shrink),
            rule::horizontal(1.0),
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
                button("Next").on_press(Next).style(button::success),
                button("Cancel").on_press(Cancel).style(button::danger)
            ]
            .spacing(10.0)
            .align_y(Vertical::Center)
        ]
        .spacing(10.0)
        .padding(padding::all(10.0))
        .align_x(Horizontal::Right)
        .into()
    }
    fn review_mod_item(&self, i: usize) -> Element<'_, Message> {
        use Message::*;
        column![
            button(
                row![
                    container(image("assets/icons/logo.png").height(50.0).width(50.0))
                        .style(container::bordered_box),
                    column![text!("Mod name"), text!("Mod Version")].spacing(5.0),
                    right_center(
                        row![button("Delete")]
                            .spacing(5.0)
                            .align_y(Vertical::Center)
                    )
                ]
                .padding(10.0)
                .spacing(10.0),
            )
            .on_press(SelectMod(i))
            .style(button::subtle)
            .width(Length::Fill),
            rule::horizontal(1.0)
        ]
        .into()
    }
    fn view_review(&self) -> Element<'_, Message> {
        use Message::*;

        column![
            //Instance details (name and icon)
            row![
                button(image("assets/icons/logo.png").height(50.0).width(50.0)).style(button::text),
                column![
                    text!("Instance Name "),
                    text!("{}", &self.instance.name),
                    text!("1.20.2")
                ]
                .spacing(5.0)
            ]
            .align_y(Vertical::Center)
            .spacing(10.0)
            .padding(padding::all(10.0))
            .height(Length::Shrink),
            rule::horizontal(1.0),
            container(text!("The following mods will be installed"))
                .padding(padding::vertical(5.0).horizontal(10.0)),
            rule::horizontal(1.0),
            scrollable(column((0..100).map(|i| self.review_mod_item(i))))
                .height(Length::Fill)
                .width(Length::Fill),
            rule::horizontal(1.0),
            right_center(
                row![
                    button("Back").on_press(Back).style(button::secondary),
                    button("Finish")
                        .on_press(CreateInstance)
                        .style(button::success),
                    button("Cancel").on_press(Cancel).style(button::danger)
                ]
                .height(Length::Shrink)
                .width(Length::Shrink)
                .align_y(Vertical::Center)
                .spacing(10.0)
                .padding(padding::all(10.0))
            )
            .height(Length::Shrink)
        ]
        .spacing(10.0)
        .into()
    }
    fn mod_item(&self, i: usize) -> Element<'_, Message> {
        use Message::*;
        column![
            button(
                row![
                    container(image("assets/icons/logo.png").height(50.0).width(50.0))
                        .style(container::bordered_box),
                    column![text!("Mod name"), text!("Mod Version")].spacing(5.0)
                ]
                .padding(10.0)
                .spacing(10.0),
            )
            .on_press(SelectMod(i))
            .style(move |theme, mut status| {
                if let Some(s) = self.selected_mod
                    && s == i
                {
                    status = button::Status::Pressed
                };
                button::Style {
                    ..button::subtle(theme, status)
                }
            })
            .width(Length::Fill),
            rule::horizontal(1.0)
        ]
        .into()
    }
    fn view_mods<'a>(&'a self, state: &'a GruntState) -> Element<'a, Message> {
        use Message::*;
        //Main container
        column![
            row![
                //Search mods
                column![
                    column![text!("Search"), text_input("Search for mods", "")]
                        .padding(padding::all(10.0))
                        .spacing(5.0),
                    rule::horizontal(1.0),
                    scrollable(column((0..100).map(|i| self.mod_item(i))))
                ]
                .width(Length::FillPortion(2)),
                rule::vertical(1.0),
                //View mods info and select
                column![
                    column!(text!("Mod Name"))
                        .spacing(5.0)
                        .padding(padding::all(10.0)),
                    rule::horizontal(1.0),
                    //Mod info
                    scrollable(web_view(&state.webview_content)).height(Length::Fill),
                    rule::horizontal(1.0),
                    row![
                        row![
                            button("Open in default browser")
                                .style(button::text)
                                .on_press(OpenInBrowser(
                                    "https://mods.vintagestory.at/algernonswatersheds".to_string()
                                ))
                        ]
                        .align_y(Vertical::Center)
                        .padding(10.0),
                        right_center(
                            row![text!("Version dropdown"), button("Add")]
                                .spacing(10.0)
                                .padding(padding::all(10.0))
                                .align_y(Vertical::Center)
                        )
                        .height(Length::Shrink)
                    ]
                    .height(Length::Shrink)
                ]
                .width(Length::FillPortion(3))
            ],
            rule::horizontal(1.0),
            right_center(
                row![
                    button("Back").on_press(Back).style(button::secondary),
                    button("Next").on_press(Next).style(button::success),
                    button("Cancel").on_press(Cancel).style(button::danger)
                ]
                .height(Length::Shrink)
                .width(Length::Shrink)
                .align_y(Vertical::Center)
                .spacing(10.0)
                .padding(padding::all(10.0))
            )
            .height(Length::Shrink)
        ]
        .into()
    }
    pub fn view<'a>(&'a self, state: &'a GruntState) -> Element<'a, Message> {
        use Message::*;
        row![
            scrollable(
                column![
                    button("1. Basics")
                        .on_press(Navigate(Step::Basic))
                        .width(Length::Fill)
                        .style(|theme, mut status| {
                            if self.step == Step::Basic {
                                status = button::Status::Pressed
                            };
                            button::subtle(theme, status)
                        }),
                    button("2. Mods")
                        .on_press(Navigate(Step::Mod))
                        .width(Length::Fill)
                        .style(|theme, mut status| {
                            if self.step == Step::Mod {
                                status = button::Status::Pressed
                            };
                            button::subtle(theme, status)
                        }),
                    button("3. Review")
                        .on_press(Navigate(Step::Review))
                        .width(Length::Fill)
                        .style(|theme, mut status| {
                            if self.step == Step::Review {
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
            match self.step {
                Step::Basic => self.view_basic(state),
                Step::Mod => self.view_mods(state),
                Step::Review => self.view_review(),
            }
        ]
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    }
    fn fetch_mod(url: String) -> Task<Message> {
        iced::Task::future(async move {
            let client = reqwest::Client::new();
            let response: serde_json::Value = client
                .get(url)
                .header("Accept", "application/json")
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            tracing::debug!("{:?}", response);
            let html = response["mod"]["text"].as_str().unwrap();
            Message::ModViewPageFetched(Ok(html.to_string()))
        })
    }
    pub fn update(&mut self, message: Message, state: &mut GruntState) -> ScreenOutput<Message> {
        use GruntAction::*;
        use Message::*;

        match message {
            Cancel => ScreenOutput::action(CloseScreen),
            NameChanged(name) => {
                self.instance.name = name;
                ScreenOutput::none()
            }
            Navigate(step) => {
                self.step = step;
                ScreenOutput::none()
            }
            Next => {
                self.step.next();
                ScreenOutput::none()
            }
            Back => {
                self.step.back();
                ScreenOutput::none()
            }
            SelectMod(i) => {
                self.selected_mod = Some(i);
                ScreenOutput::task(Self::fetch_mod(
                    "https://mods.vintagestory.at/api/mod/7286".to_string(),
                ))
            }
            Message::CreateInstance => {
                ScreenOutput::action(GruntAction::CreateInstance(self.instance.clone()))
                    .action_add(CloseScreen)
            }
            ModViewPageFetched(Ok(page)) => {
                state.webview_content.load_html(&page);
                ScreenOutput::none()
            }
            OpenInBrowser(url) => {
                let _ = webbrowser::open(&url);
                ScreenOutput::none()
            }
            _ => ScreenOutput::none(),
        }
    }
}
