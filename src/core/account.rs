#[derive(PartialEq, Eq, Clone, Debug, Copy)]
pub enum AccountStatus {
    Ok,
    Expired,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Account {
    pub username: String,
    pub email: String,
    pub status: AccountStatus,
}

impl Account {
    pub fn new(username: &str, email: &str, status: AccountStatus) -> Self {
        Self {
            username: username.into(),
            email: email.into(),
            status,
        }
    }
}
