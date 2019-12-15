#![allow(dead_code)]

mod channels;
mod database;
mod media;
mod messages;
mod spaces;
mod users;

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().unwrap();
    Ok(())
}
