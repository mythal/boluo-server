use clap::Clap;
use postgres::{Client, NoTls};

#[derive(Clap)]
#[clap(version = "0.1", author = "Coppa <sortal@protonmail.com>")]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    Init(Init),
}

#[derive(Clap)]
struct Init {
    database_url: Option<String>,
}

fn main() -> Result<(), anyhow::Error> {
    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::Init(Init { database_url }) => {
            println!("initializing database");

            let database_url = database_url.or(std::env::var("DATABASE_URL").ok()).unwrap();

            let mut client = Client::connect(&database_url, NoTls)?;
            client.batch_execute(include_str!("../schema.sql"))?;
        }
    }
    Ok(())
}