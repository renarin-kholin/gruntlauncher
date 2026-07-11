use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    assets::VSAUTH,
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
    let data_dir = paths::data_dir()?;
    tokio::fs::create_dir_all(&data_dir).await?;
    let session_path = data_dir.join("session.toml");
    let session_file_content = tokio::fs::read_to_string(session_path).await?;
    //TODO: check if the session is valid or expired
    Ok(toml::from_str(&session_file_content)?)
}
pub async fn save_session(account: Account) -> Result<()> {
    let data_dir = paths::data_dir()?;
    tokio::fs::create_dir_all(&data_dir).await?;
    let session_path = data_dir.join("session.toml");
    let mut accounts_store = load_session().await.unwrap_or_default();
    if let Some(existing) = accounts_store
        .accounts
        .iter_mut()
        .find(|a| a.email == account.email)
    {
        *existing = account.clone();
    } else {
        accounts_store.accounts.push(account.clone());
    }
    accounts_store.selected_account = Some(account.username.clone());
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
            save_session(account.clone()).await?;
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
