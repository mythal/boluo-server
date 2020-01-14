use serde::Deserialize;

#[derive(Deserialize)]
pub struct Upload {
    pub filename: String,
    pub mine_type: String,
}
