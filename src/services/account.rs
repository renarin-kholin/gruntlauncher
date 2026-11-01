use std::{collections::HashMap, io, path::Path, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    assets::{VSAUTH, VSAUTHVALIDATE},
    core::account::{Account, AccountStatus, AccountStore},
    paths::{self, ProjectDirError},
    services::HTTP,
};
use thiserror::Error;
use tracing::{debug, error};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionResponse {
    pub sessionkey: Option<String>,
    pub sessionsignature: Option<String>,
    pub mptoken: Option<serde_json::Value>,
    pub uid: Option<String>,
    pub entitlements: Option<serde_json::Value>,
    pub playername: Option<String>,
    pub hasgameserver: Option<bool>,
    pub valid: i64,
    pub prelogintoken: Option<String>,
    pub reason: Option<String>,
}

#[derive(Error, Debug, Clone)]
pub enum AccountsError {
    #[error("io error: {0}")]
    Io(Arc<std::io::Error>),

    #[error("request to the server failed: {0}")]
    Reqwest(Arc<reqwest::Error>),

    #[error("Failed when trying to get session details from login response.")]
    ParseError,

    #[error(transparent)]
    ProjectDir(#[from] ProjectDirError),

    #[error("error when trying to deserialize config file: {0}")]
    DeserializeError(#[from] toml::de::Error),

    #[error("error when trying to serialize config file: {0}")]
    SerializeError(#[from] toml::ser::Error),

    #[error(transparent)]
    SerdeJson(Arc<serde_json::Error>),
}
impl From<reqwest::Error> for AccountsError {
    fn from(value: reqwest::Error) -> Self {
        AccountsError::Reqwest(Arc::new(value))
    }
}
impl From<std::io::Error> for AccountsError {
    fn from(value: std::io::Error) -> Self {
        AccountsError::Io(Arc::new(value))
    }
}
impl From<serde_json::Error> for AccountsError {
    fn from(value: serde_json::Error) -> Self {
        AccountsError::SerdeJson(Arc::new(value))
    }
}
pub type Result<T> = std::result::Result<T, AccountsError>;

type PreLoginToken = String;
#[derive(Clone, Debug)]
pub enum LoginStatus {
    Success(Account),
    NeedTOTP(PreLoginToken),
    WrongDetails,
    IPChanged,
    TemporarilyBlocked,
    Failed,
}
pub async fn load_session() -> Result<AccountStore> {
    // TODO: check if the session is valid or expired.
    load_session_from(&paths::data_dir()?.join("session.toml")).await
}

async fn load_session_from(session_path: &Path) -> Result<AccountStore> {
    match tokio::fs::read_to_string(session_path).await {
        Ok(contents) => Ok(toml::from_str(&contents)?),
        // No store exists until the user saves their first account.
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(AccountStore::default()),
        Err(error) => Err(error.into()),
    }
}
pub async fn save_account(account: Account) -> Result<()> {
    let data_dir = paths::data_dir()?;
    tokio::fs::create_dir_all(&data_dir).await?;
    let session_path = data_dir.join("session.toml");
    let mut accounts_store = load_session().await?;
    if let Some(existing) = accounts_store
        .accounts
        .iter_mut()
        .find(|a| a.username == account.username)
    {
        *existing = account.clone();
    } else {
        accounts_store.accounts.push(account.clone());
    }
    accounts_store.selected_account = Some(account.username.clone());
    tokio::fs::write(session_path, toml::to_string(&accounts_store)?).await?;
    Ok(())
}
pub async fn save_accounts(accounts_store: AccountStore) -> Result<()> {
    let data_dir = paths::data_dir()?;
    tokio::fs::create_dir_all(&data_dir).await?;
    let session_path = data_dir.join("session.toml");
    tokio::fs::write(session_path, toml::to_string(&accounts_store)?).await?;
    Ok(())
}

pub async fn send_login(
    email: String,
    password: String,
    totp: Option<String>,
    prelogintoken: Option<String>,
) -> Result<LoginStatus> {
    let mut params = HashMap::new();
    params.insert("email", email.clone());
    params.insert("password", password);
    if let (Some(totp), Some(prelogintoken)) = (totp, prelogintoken) {
        params.insert("totpcode", totp);
        params.insert("prelogintoken", prelogintoken);
    }
    let response = HTTP.post(VSAUTH).form(&params).send().await?;
    let parsed_response: SessionResponse = response.json().await?;
    match parsed_response.valid {
        1 => {
            let account = Account::new(
                &parsed_response
                    .playername
                    .ok_or(AccountsError::ParseError)?,
                &email,
                AccountStatus::Ok,
                parsed_response
                    .sessionkey
                    .ok_or(AccountsError::ParseError)?,
                parsed_response
                    .sessionsignature
                    .ok_or(AccountsError::ParseError)?,
                parsed_response.uid.ok_or(AccountsError::ParseError)?,
            );
            save_account(account.clone()).await?;
            Ok(LoginStatus::Success(account))
        }
        _ => {
            if let Some(ref reason) = parsed_response.reason {
                match (reason.as_str(), parsed_response.prelogintoken.clone()) {
                    ("requiretotpcode", Some(prelogintoken)) => {
                        Ok(LoginStatus::NeedTOTP(prelogintoken))
                    }
                    ("invalidemailorpassword", None) => Ok(LoginStatus::WrongDetails),
                    ("ipchanged", None) => Ok(LoginStatus::IPChanged),
                    ("temporarilyblocked", None) => Ok(LoginStatus::TemporarilyBlocked),
                    (_, _) => {
                        debug!("{:?}", parsed_response);
                        Ok(LoginStatus::Failed)
                    }
                }
            } else {
                error!("{:?}", parsed_response);
                Ok(LoginStatus::Failed)
            }
        }
    }
}
#[derive(Deserialize, Serialize, Debug)]
struct ClientValidateResponse {
    valid: i32,
}
pub async fn validate_session(account: Account) -> Result<bool> {
    let mut params = HashMap::new();
    params.insert("uid", account.uid);
    params.insert("sessionkey", account.sessionkey);

    let response = HTTP.get(VSAUTHVALIDATE).form(&params).send().await?;
    let response_text = response.text().await?;
    let parsed_response: ClientValidateResponse = serde_json::from_str(&response_text)?;
    Ok(parsed_response.valid == 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct SessionDir(PathBuf);

    impl SessionDir {
        fn new() -> Self {
            let path = std::env::temp_dir()
                .join(format!("gruntlauncher-accounts-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for SessionDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[tokio::test]
    async fn missing_session_is_an_empty_store_without_creating_files() {
        let directory = SessionDir::new();
        for path in [
            directory.0.join("session.toml"),
            directory.0.join("new-user/session.toml"),
        ] {
            assert_eq!(
                load_session_from(&path).await.unwrap(),
                AccountStore::default()
            );
            assert!(!path.exists());
        }
        assert!(!directory.0.join("new-user").exists());
    }

    #[tokio::test]
    async fn existing_accounts_and_selected_account_are_loaded() {
        let directory = SessionDir::new();
        let path = directory.0.join("session.toml");
        let store = AccountStore {
            accounts: vec![Account::new(
                "test-player",
                "test@example.invalid",
                AccountStatus::Ok,
                "test-session-key".into(),
                "test-signature".into(),
                "test-uid".into(),
            )],
            selected_account: Some("test-player".into()),
        };
        let contents = toml::to_string(&store).unwrap();
        tokio::fs::write(&path, &contents).await.unwrap();
        assert_eq!(load_session_from(&path).await.unwrap(), store);
        assert_eq!(tokio::fs::read_to_string(path).await.unwrap(), contents);
    }

    #[tokio::test]
    async fn invalid_stores_and_other_read_errors_are_not_treated_as_first_run() {
        let directory = SessionDir::new();
        let path = directory.0.join("session.toml");
        let contents = "accounts = [invalid TOML";
        tokio::fs::write(&path, contents).await.unwrap();
        assert!(matches!(
            load_session_from(&path).await,
            Err(AccountsError::DeserializeError(_))
        ));
        assert_eq!(tokio::fs::read_to_string(path).await.unwrap(), contents);
        // Reading a directory fails on Unix and Windows, without relying on file permissions.
        assert!(matches!(
            load_session_from(&directory.0).await,
            Err(AccountsError::Io(_))
        ));
    }
}
