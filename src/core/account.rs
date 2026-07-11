use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Clone, Debug, Copy, Serialize, Deserialize)]
pub enum AccountStatus {
    Ok,
    Expired,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Account {
    pub username: String,
    pub email: String,
    pub status: AccountStatus,
    pub sessionkey: String,
    pub sessionsignature: String,
    pub uid: String,
}

impl Account {
    pub fn new(
        username: &str,
        email: &str,
        status: AccountStatus,
        sessionkey: String,
        sessionsignature: String,
        uid: String,
    ) -> Self {
        Self {
            username: username.into(),
            email: email.into(),
            status,
            sessionkey,
            sessionsignature,
            uid,
        }
    }
}
pub type Username = String;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AccountStore {
    pub accounts: Vec<Account>,
    pub selected_account: Option<Username>,
}
