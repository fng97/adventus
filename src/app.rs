use crate::event_handler::Handler;
use crate::player::HttpKey;
use reqwest::Client as HttpClient;
use serenity::{client::Client, prelude::GatewayIntents};
use songbird::SerenityInit;
use sqlx::PgPool;

pub async fn build(discord_token: String, pool: PgPool) -> Client {
    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_VOICE_STATES;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to migrate the database");

    let handler = Handler { database: pool };

    Client::builder(discord_token, intents)
        .event_handler(handler)
        .register_songbird()
        .type_map_insert::<HttpKey>(HttpClient::new()) // shared HTTP client for YoutubeDl
        .await
        .expect("Error creating client")
}
