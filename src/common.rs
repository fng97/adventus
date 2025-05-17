pub struct Data {
    // pub http_client: reqwest::Client,
    // pub database: sqlx::PgPool,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;
