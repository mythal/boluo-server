use once_cell::sync::OnceCell;
use postgres_types::FromSql;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::database::{Client, CreationError, FetchError, SqlType};

#[derive(Debug, Serialize, Deserialize, FromSql)]
#[postgres(name = "users")]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub nickname: String,
    pub password: String,
    pub bio: String,
    pub joined: chrono::naive::NaiveDateTime,
    pub deactivated: bool,
    pub avatar_id: Option<Uuid>,
}

type StatementCell = OnceCell<postgres::Statement>;

impl User {
    pub fn all(db: &mut Client) -> Result<Vec<User>, postgres::Error> {
        let statement = db.prepare(include_str!("select_users.sql"), &[]);

        db.client
            .query(&statement, &[])
            .map(|result_set| result_set.into_iter().map(|row| row.get(0)).collect())
    }

    pub fn create(db: &mut Client, email: &str, username: &str, nickname: &str, password: &str) -> Result<User, CreationError> {
        let statement = db.prepare(include_str!("create_user.sql"), &[SqlType::TEXT, SqlType::TEXT, SqlType::TEXT, SqlType::TEXT]);

        db.client
            .query(&statement, &[&email, &username, &nickname, &password])?
            .into_iter()
            .next() // first row
            .map(|row| row.get(0))
            .ok_or(CreationError::AlreadyExists)
    }

    fn get(db: &mut Client, id: Option<&Uuid>, email: Option<&str>, username: Option<&str>) -> Result<User, FetchError> {
        let statement = db.prepare(include_str!("fetch_user.sql"), &[SqlType::UUID, SqlType::TEXT, SqlType::TEXT]);

        db.client
            .query(&statement, &[&id, &email, &username])?
            .into_iter()
            .next()
            .ok_or(FetchError::NoSuchRecord)
            .map(|row| row.get(0))
    }

    pub fn get_by_id(db: &mut Client, id: &Uuid) -> Result<User, FetchError> {
        User::get(db, Some(id), None, None)
    }

    pub fn get_by_email(db: &mut Client, email: &str) -> Result<User, FetchError> {
        User::get(db, None, Some(email), None)
    }

    pub fn get_by_username(db: &mut Client, username: &str) -> Result<User, FetchError> {
        User::get(db, None, None, Some(username))
    }

    pub fn delete_by_id(db: &mut Client, id: &Uuid) -> Result<User, FetchError> {
        let statement = db.prepare(include_str!("delete_user.sql"), &[SqlType::UUID]);

        db.client
            .query(&statement, &[id])?
            .into_iter()
            .next()
            .ok_or(FetchError::NoSuchRecord)
            .map(|row| row.get(0))
    }
}

#[test]
fn user_test() {
    let mut client = Client::new();
    let email = "humura@humura.net";
    let username = "humura";
    let nickname = "Akami Humura";
    let password = "MadokaMadokaSuHaSuHa";
    let new_user = User::create(&mut client, email, username, nickname, password).unwrap();
    let user = User::get_by_id(&mut client, &new_user.id).unwrap();
    assert_eq!(user.email, email);

    let mut client = Client::new();
    let user = User::get_by_email(&mut client, &new_user.email).unwrap();
    assert_eq!(user.nickname, nickname);

    let deleted_user = User::delete_by_id(&mut client, &new_user.id).unwrap();
    assert_eq!(deleted_user.id, user.id);

    let all_users = User::all(&mut client).unwrap();
    assert!(all_users.into_iter().find(|u| u.id == user.id).is_none());
}
