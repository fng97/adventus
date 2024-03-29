use serenity::model::id::GuildId;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

pub struct Data {
    pub http_client: reqwest::Client,
    pub database: sqlx::PgPool,
    pub last_active: Arc<Mutex<HashMap<GuildId, Instant>>>,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;
