use chrono::naive::NaiveDateTime;
use postgres_types::FromSql;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::database::{Client, CreationError, FetchError, SqlType};

#[derive(Debug, Serialize, Deserialize, FromSql)]
#[postgres(name = "spaces")]
pub struct Space {
    id: Uuid,
    name: String,
    description: String,
    created: NaiveDateTime,
    modified: NaiveDateTime,
    owner_id: Uuid,
    is_public: bool,
    deleted: bool,
    password: String,
    language: String,
    default_dice_type: String,
}


impl Space {
    fn create(db: &mut Client, name: &str, owner_id: &Uuid, password: Option<&str>) -> Result<Space, CreationError> {
        let statement = db
            .prepare(include_str!("create_space.sql"), &[SqlType::TEXT, SqlType::UUID, SqlType::TEXT]);
        db.client
            .query(&statement, &[&name, owner_id, &password])?
            .into_iter()
            .next()
            .map(|row| row.get(0))
            .ok_or(CreationError::AlreadyExists)
    }

    fn delete(db: &mut Client, id: &Uuid) -> Result<Space, FetchError> {
        let statement = db
            .prepare(include_str!("delete_space.sql"), &[SqlType::UUID]);
        db.client
            .query(&statement, &[id])?
            .into_iter()
            .next()
            .map(|row| row.get(0))
            .ok_or(FetchError::NoSuchRecord)
    }

    fn get(db: &mut Client, id: Option<&Uuid>, name: Option<&str>) -> Result<Space, FetchError> {
        let statement = db
            .prepare(include_str!("fetch_space.sql"), &[SqlType::UUID, SqlType::TEXT, SqlType::BOOL]);

        db.client
            .query(&statement, &[&id, &name, &false])?
            .into_iter()
            .next()
            .map(|row| row.get(0))
            .ok_or(FetchError::NoSuchRecord)
    }

    pub fn get_by_id(db: &mut Client, id: &Uuid) -> Result<Space, FetchError> {
        Space::get(db, Some(id), None)
    }

    pub fn get_by_name(db: &mut Client, name: &str) -> Result<Space, FetchError> {
        Space::get(db, None, Some(name))
    }
}

#[test]
fn space_test() {
    use crate::users::User;
    let mut client = Client::new();
    let email = "spaces@mythal.net";
    let username = "space_test";
    let password = "no password";
    let nickname = "Space Test";
    let space_name = "Pure Illusion";
    let user = User::create(&mut client, email, username, nickname, password).unwrap();
    let space = Space::create(&mut client, space_name, &user.id, None).unwrap();
    let space = Space::get_by_name(&mut client, &space.name).unwrap();
    let space = Space::get_by_id(&mut client, &space.id).unwrap();
    let space = Space::delete(&mut client, &space.id).unwrap();
    assert_eq!(space.name, space_name);
    User::delete_by_id(&mut client, &user.id).unwrap();
}

