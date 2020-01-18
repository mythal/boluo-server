use super::User;
use crate::database::Querist;
use crate::AppError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct QueryUser {
    pub id: Option<Uuid>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Register {
    pub email: String,
    pub username: String,
    pub nickname: String,
    pub password: String,
}

impl Register {
    pub async fn register<T: Querist>(&self, db: &mut T) -> Result<User, AppError> {
        User::create(db, &*self.email, &*self.username, &*self.nickname, &*self.password).await
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Login {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub with_token: bool,
}

impl Login {
    pub async fn login<T: Querist>(&self, db: &mut T) -> Result<User, AppError> {
        User::login(db, None, Some(&self.username), &*self.password).await
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginReturn {
    pub user: User,
    pub token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Edit {
    pub nickname: Option<String>,
    pub bio: Option<String>,
    pub avatar: Option<Uuid>,
}

impl Edit {
    pub fn validate(&self) -> Result<(), &'static str> {
        use crate::validators::{BIO, NICKNAME};

        if let Some(ref nickname) = self.nickname {
            NICKNAME.run(nickname)?;
        }
        if let Some(ref bio) = self.bio {
            BIO.run(bio)?;
        }
        Ok(())
    }
}
