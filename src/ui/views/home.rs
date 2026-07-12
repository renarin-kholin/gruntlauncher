use email_address::EmailAddress;
use iced::{
    Element, Length, Task,
    alignment::{Horizontal, Vertical},
    padding,
    widget::{
        self, Column, button, center, center_x, column, image::Handle, right, row, rule,
        scrollable, space, text, text_input,
    },
};
use tracing::{debug, error, info};

use crate::{
    assets::GRUNT_ICON,
    core::{
        account::{AccountStatus, AccountStore},
        instance::{GruntInstance, InstanceId},
    },
    services::{
        account::{
            AccountsError, LoginStatus, save_account, save_accounts, send_login, validate_session,
        },
        instance::{self, InstancesError},
    },
    ui::{
        GruntAction, GruntState,
        views::{ScreenOutput, home::Message::LoginResult},
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

    //Set while confirming removal of an account; drives the confirmation modal.
    pending_remove: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectInstance(InstanceId),
    LaunchInstance,
    InstanceLaunched(Result<(), InstancesError>),
    AddInstance,

    //Login Related
    AccountSelected(String),
    AccountRemoveRequested(String),
    ConfirmAccountRemove,
    CancelAccountRemove,
    LoginRequested(LoginRequest),
    EmailChange(String),
    PasswordChange(String),
    TOTPChange(String),
    DoLogin,
    CancelLogin,
    SessionSaved(Result<(), AccountsError>),
    SessionValidated(Result<bool, AccountsError>),
    LoginResult(Result<LoginStatus, AccountsError>),

    //Folders
    OpenInstanceFolder,
    OpenModFolder,

    ApplyUpdate,
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
            pending_remove: None,
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
    pub fn view_login<'a>(&'a self, _state: &'a GruntState) -> Element<'a, Message> {
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
    pub fn view_remove_confirm<'a>(&'a self, username: &str) -> Element<'a, Message> {
        use Message::*;
        let content = column![
            text!("Remove {}?", username),
            text!("You'll need to sign in again to use this account."),
            space().height(20.0),
            button(center_x("Remove"))
                .width(Length::Fill)
                .style(button::danger)
                .on_press(ConfirmAccountRemove),
            button(center_x("Cancel"))
                .width(Length::Fill)
                .style(button::subtle)
                .on_press(CancelAccountRemove),
        ]
        .spacing(5.0);
        center(row![
            space().width(Length::FillPortion(1)),
            content.width(Length::FillPortion(2)),
            space().width(Length::FillPortion(1))
        ])
        .into()
    }
    pub fn view_sidebar<'a>(&'a self, state: &'a GruntState) -> Element<'a, Message> {
        use Message::*;
        let mut sidebar: Column<'a, Message> = column![]
            .align_x(Horizontal::Center)
            .width(Length::FillPortion(1))
            .spacing(10.0)
            .padding(padding::all(10.0));
        if let Some(selected_instance) = self.selected_instance
            && let Some(i) = state.instances.iter().find(|i| i.id == selected_instance)
        {
            sidebar = sidebar
                .push(
                    widget::image(self.icon_handles[0].clone())
                        .height(80.0)
                        .width(80.0),
                )
                .push(text!("{}", i.name).wrapping(text::Wrapping::Glyph).center());
        } else {
            sidebar = sidebar.push(text!("No instance selected"));
        }
        sidebar
            .push(
                button(center_x("Launch"))
                    .width(Length::Fill)
                    .on_press_maybe(self.selected_instance.map(|_| LaunchInstance)),
            )
            .push(button(center_x("Edit")).width(Length::Fill))
            .push(rule::horizontal(1.0))
            .push(text!("Open Folders"))
            .push(
                button(center_x("Mods"))
                    .width(Length::Fill)
                    .on_press_maybe(self.selected_instance.map(|_| OpenModFolder)),
            )
            .push(
                button(center_x("Instance"))
                    .width(Length::Fill)
                    .on_press_maybe(self.selected_instance.map(|_| OpenInstanceFolder)),
            )
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
                    .on_remove(AccountRemoveRequested),
            )
        };
        let base = column![
            //Topbar
            row![
                button("Add Instance").on_press(Message::AddInstance),
                rule::vertical(2),
                button("Settings"),
                right({
                    let mut right_side = row![].spacing(10.0).align_y(Vertical::Center);
                    if let Some(update) = &state.available_update {
                        right_side = right_side.push(
                            button(text!("Update to v{}", update.TargetFullRelease.Version))
                                .style(button::success)
                                .on_press(ApplyUpdate),
                        );
                    }
                    right_side.push(accounts)
                })
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
                self.view_sidebar(state)
            ]
        ]
        .height(Length::Fill)
        .width(Length::Fill);
        let (panel_children, panel_title) = if let Some(username) = &self.pending_remove {
            (
                Some(self.view_remove_confirm(username)),
                Some("Remove account".to_string()),
            )
        } else if self.show_login {
            (Some(self.view_login(state)), Some("Login".to_string()))
        } else {
            (None, None)
        };
        overlay_container(base.into(), panel_children, panel_title)
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
                    return ScreenOutput::task(Task::batch([
                        Task::perform(save_account(account.clone()), SessionSaved),
                        Task::perform(validate_session(account.clone()), SessionValidated),
                    ]));
                }
            }
            AccountRemoveRequested(username) => {
                self.pending_remove = Some(username);
            }
            CancelAccountRemove => {
                self.pending_remove = None;
            }
            ConfirmAccountRemove => {
                if let Some(username) = self.pending_remove.take() {
                    state.accounts.retain(|a| a.username != username);
                    if state.selected_account.as_deref() == Some(username.as_str()) {
                        state.selected_account = state.accounts.first().map(|a| a.username.clone());
                    }
                    return ScreenOutput::task(Task::perform(
                        save_accounts(AccountStore {
                            accounts: state.accounts.clone(),
                            selected_account: state.selected_account.clone(),
                        }),
                        SessionSaved,
                    ));
                }
            }
            LoginRequested(login_request) => {
                use LoginRequest::*;
                match login_request {
                    LoginRequest::AddAccount => {
                        self.login_details.clear();
                        self.show_login = true;
                    }
                    Relogin(email) => {
                        self.login_details.clear();
                        self.login_details.email = email;
                        self.show_login = true;
                    }
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
                return self.add_account(state);
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
                                if let Some(existing) = state
                                    .accounts
                                    .iter_mut()
                                    .find(|a| a.username == account.username)
                                {
                                    *existing = account;
                                } else {
                                    state.accounts.push(account);
                                }
                                self.login_details.clear();
                                self.show_login = false;
                            }
                            NeedTOTP(prelogintoken) => {
                                self.login_details.prelogintoken = Some(prelogintoken);
                                self.login_details.error = Some(
                                    "Please enter your two factor code to continue.".to_string(),
                                );
                            }
                            WrongDetails => {
                                self.login_details.error =
                                    Some("Invalid email or password".to_string());
                            }
                            IPChanged => {
                                self.login_details.error = Some("IP change was detected and relogin is required. Please try again.".to_string());
                            }
                            TemporarilyBlocked => {
                                self.login_details.error = Some("You are temporarily blocked on the auth server for repeated logins. Try again later.".to_string());
                            }
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
            SessionValidated(result) => match result {
                Ok(true) => info!("Session is valid."),
                Ok(false) => {
                    if let Some(username) = &state.selected_account
                        && let Some(account) =
                            state.accounts.iter_mut().find(|a| a.username == *username)
                    {
                        account.status = AccountStatus::Expired;
                        self.login_details.clear();
                        self.show_login = true;
                        self.login_details.error =
                            Some("Your login session has expired, login again.".into());
                        self.login_details.email = account.email.clone();
                        return ScreenOutput::task(Task::perform(
                            save_account(account.clone()),
                            SessionSaved,
                        ));
                    }
                }
                Err(e) => error!("{e}"),
            },
            OpenInstanceFolder => {
                if let Some(selected_instance) = self.selected_instance {
                    let instances_path = state.config.instances_folder.clone();
                    let instance_folder = instances_path.join(selected_instance.to_string());
                    match open::that(&instance_folder) {
                        Ok(()) => {
                            info!("Opened {:?} in the default application.", instance_folder);
                        }
                        Err(e) => {
                            error!("Error when trying to open a folder: {e}");
                        }
                    }
                }
            }
            OpenModFolder => {
                if let Some(selected_instance) = self.selected_instance {
                    let instances_path = state.config.instances_folder.clone();
                    let mod_folder = instances_path
                        .join(selected_instance.to_string())
                        .join("Mods");
                    match open::that(&mod_folder) {
                        Ok(()) => {
                            info!("Opened {:?} in the default application.", mod_folder);
                        }
                        Err(e) => {
                            error!("Error when trying to open a folder: {e}");
                        }
                    }
                }
            }
            Message::ApplyUpdate => {
                return ScreenOutput::action(GruntAction::ApplyUpdate);
            }
        }
        ScreenOutput::none()
    }
    fn add_account(&mut self, _state: &mut GruntState) -> ScreenOutput<Message> {
        self.login_details.error = None;
        self.show_login = true;
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
        ScreenOutput::task(Task::perform(
            send_login(
                self.login_details.email.clone(),
                self.login_details.password.clone(),
                totp,
                self.login_details.prelogintoken.clone(),
            ),
            LoginResult,
        ))
    }
}
