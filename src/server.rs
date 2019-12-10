use std::env;

mod database;
mod users;
mod spaces;


fn main() -> anyhow::Result<()> {
    dotenv::dotenv().unwrap();
    Ok(())
}
