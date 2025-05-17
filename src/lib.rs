pub mod introductions;
pub mod rolls;

pub struct Config {
    pub intros_dir: std::path::PathBuf,
}

pub struct Data {
    pub config: Config,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;
