use email_address::EmailAddress;
use iced::{
    Element, Length, Task,
    alignment::{Horizontal, Vertical},
    padding,
    theme::palette::Success,
    widget::{
        self, button, center, center_x, column, image::Handle, right, row, rule, scrollable, space,
        text, text_input,
    },
};
use tracing::{debug, error, info};

use crate::{
    assets::GRUNT_ICON,
    core::{
        account::Account,
        instance::{GruntInstance, InstanceId},
    },
    services::{
        account::{AccountsError, LoginStatus, save_session, send_login},
        instance::{self, InstancesError},
    },
    ui::{
        GruntAction, GruntState,
        views::ScreenOutput,
        widget::{
            account::{self, LoginRequest},
            overlay::overlay_container,
        },
    },
};
#[derive(Default, Debug)]
struct LoginDetail {
    email: String,
    password: String,
    totp: String,
    prelogintoken: Option<String>,
    error: Option<String>,
}

impl LoginDetail {
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}
pub struct Screen {
    selected_instance: Option<InstanceId>,
    icon_handles: Vec<Handle>,

    //Login form state
    show_login: bool,
    login_details: LoginDetail,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectInstance(InstanceId),
    LaunchInstance,
    InstanceLaunched(Result<(), InstancesError>),
    AddInstance,

    //Login Related
    AccountSelected(String),
    AccountRemove(String),
    LoginRequested(LoginRequest),
    EmailChange(String),
    PasswordChange(String),
    TOTPChange(String),
    DoLogin,
    CancelLogin,
    SessionSaved(Result<(), AccountsError>),

    LoginResult(Result<LoginStatus, AccountsError>),
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

            //Login Fields
            show_login: false,
            login_details: LoginDetail::default(),
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
    pub fn view_login<'a>(&'a self, state: &'a GruntState) -> Element<'a, Message> {
        use Message::*;
        let mut form = column![
            text!("Email"),
            text_input("example@email.com", &self.login_details.email).on_input(EmailChange),
            space().height(10.0),
            text!("Password"),
            text_input("**********", &self.login_details.password)
                .secure(true)
                .on_input(PasswordChange),
        ];
        if let Some(error) = &self.login_details.error {
            form = form.push(text!("{}", error));
        }
        if self.login_details.prelogintoken.is_some() {
            form = form
                .push(text!("Two factor code"))
                .push(text_input("123456", &self.login_details.totp).on_input(TOTPChange));
        }

        form = form
            .push(space().height(20.0))
            .push(
                button(center_x("Login"))
                    .width(Length::Fill)
                    .style(button::success)
                    .on_press(DoLogin),
            )
            .push(
                button(center_x("Cancel"))
                    .width(Length::Fill)
                    .style(button::subtle)
                    .on_press(CancelLogin),
            );
        center(row![
            space().width(Length::FillPortion(1)),
            form.spacing(5.0).width(Length::FillPortion(2)),
            space().width(Length::FillPortion(1))
        ])
        .into()
    }
    pub fn view<'a>(&'a self, state: &'a GruntState) -> Element<'a, Message> {
        use Message::*;
        let mut accounts = column![];
        accounts = if state.accounts.is_empty() {
            accounts.push(button("Sign In").on_press(LoginRequested(LoginRequest::AddAccount)))
        } else {
            accounts.push(
                account::AccountSwitcher::new(&state.accounts, state.selected_account.as_deref())
                    .on_login(LoginRequested)
                    .on_select(AccountSelected)
                    .on_remove(AccountRemove),
            )
        };
        let base = column![
            //Topbar
            row![
                button("Add Instance").on_press(Message::AddInstance),
                rule::vertical(2),
                button("Settings"),
                right(accounts)
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
        .width(Length::Fill);
        let panel_children = if self.show_login {
            Some(self.view_login(state))
        } else {
            None
        };
        overlay_container(base.into(), panel_children, Some("Login".to_string()))
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
                if let Some(selected_instance) = self.selected_instance {
                    let instance = state
                        .instances
                        .iter()
                        .find(|i| i.id == selected_instance)
                        .expect("Could not select instance");

                    return ScreenOutput::task(Task::perform(
                        instance::launch_instance(
                            instance.clone(),
                            state.config.instances_folder.clone(),
                            state.selected_account.as_ref().and_then(|s| {
                                state.accounts.iter().find(|a| a.username == *s).cloned()
                            }),
                        ),
                        InstanceLaunched,
                    ));
                }
            }
            InstanceLaunched(result) => {
                debug!("{result:?}");
            }
            AccountSelected(username) => {
                state.selected_account = Some(username.clone());
                if let Some(account) = state.accounts.iter().find(|a| a.username == username) {
                    return ScreenOutput::task(Task::perform(
                        save_session(account.clone()),
                        SessionSaved,
                    ));
                }
            }
            AccountRemove(username) => {
                state.accounts.retain(|a| a.username != username);
                if state.selected_account.as_deref() == Some(username.as_str()) {
                    state.selected_account = state.accounts.first().map(|a| a.username.clone());
                }
            }
            LoginRequested(login_request) => {
                use LoginRequest::*;
                match login_request {
                    AddAccount => {
                        self.show_login = true;
                        self.login_details.clear();
                    }
                    Relogin(email) => {}
                }
            }
            EmailChange(email) => {
                self.login_details.email = email;
            }
            PasswordChange(password) => {
                self.login_details.password = password;
            }
            TOTPChange(totp) => {
                self.login_details.totp = totp;
            }
            DoLogin => {
                self.login_details.error = None;
                debug!("Login");
                //Validate email, totp and then
                if !EmailAddress::is_valid(&self.login_details.email) {
                    self.login_details.error = Some("Invalid email. Please try again.".to_string());
                    return ScreenOutput::none();
                }
                let totp = if self.login_details.prelogintoken.is_some() {
                    Some(self.login_details.totp.clone())
                } else {
                    None
                };
                return ScreenOutput::task(Task::perform(
                    send_login(
                        self.login_details.email.clone(),
                        self.login_details.password.clone(),
                        totp,
                        self.login_details.prelogintoken.clone(),
                    ),
                    LoginResult,
                ));
            }
            CancelLogin => {
                self.login_details.clear();
                self.show_login = false;
            }
            LoginResult(result) => {
                match result {
                    Ok(login_status) => {
                        use LoginStatus::*;
                        match login_status {
                            Success(account) => {
                                //Add session details to the state,
                                state.accounts.push(account);
                                self.login_details.clear();
                                self.show_login = false;
                            }
                            NeedTOTP(prelogintoken) => {
                                self.login_details.prelogintoken = Some(prelogintoken);
                                self.login_details.error = Some(
                                    "Please enter your two factor code to continue.".to_string(),
                                );
                            }
                            WrongDetails => {}
                            IPChanged => {}
                            TemporarilyBlocked => {}
                            Failed => {
                                self.login_details.error =
                                    Some("Login failed for unknown reason.".to_string());
                            }
                        }
                    }
                    Err(e) => self.login_details.error = Some(e.to_string()),
                }
            }
            SessionSaved(result) => match result {
                Ok(()) => info!("Updated session"),
                Err(e) => error!("{e}"),
            },
        }
        ScreenOutput::none()
    }
}
