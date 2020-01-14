use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct EventQuery {
    pub mailbox: Uuid,
    pub since: i64,
}
