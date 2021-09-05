use clap::ArgMatches;
use clap::{App, Arg};
use once_cell::sync::Lazy;

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    dotenv::dotenv().ok();

    let matches = App::new("event-logger")
        .version(env!("CARGO_PKG_VERSION"))
        .about("log DCS events to Postgres")
        .arg(
            Arg::with_name("DATABASE_URL")
                .long("databse-url")
                .short("db")
                .help("the URL for the PostgreSQL the events should be persisted to")
                .env("DATABASE_URL")
                .takes_value(true),
        )
        .get_matches();

    Config(matches)
});

pub struct Config(ArgMatches<'static>);

impl Config {
    pub fn database_url(&self) -> &str {
        self.0
            .value_of("DATABASE_URL")
            .expect("DATABASE_URL is required")
    }
}
