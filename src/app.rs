use crate::commands::{self, on_error};
use crate::database::migrate;
use crate::handlers::Handler;
use crate::player::HttpKey;
use reqwest::Client as HttpClient;
use serenity::{client::Client, prelude::GatewayIntents};
use songbird::SerenityInit;
use sqlx::PgPool;
use tracing::info;

pub async fn build(discord_token: String, pool: PgPool) -> Client {
    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_VOICE_STATES;

    migrate(&pool).await;

    // TODO: Find a cleaner way to share these resources
    let handler_pool = pool.clone();
    let poise_pool = pool.clone();

    let handler = Handler {
        database: handler_pool,
    };

    let options = poise::FrameworkOptions {
        commands: vec![commands::set_intro(), commands::clear_intro()],
        on_error: |error| Box::pin(on_error(error)),
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(move |ctx, ready, framework| {
            Box::pin(async move {
                info!("Setting up commands for {}", ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(commands::Data {
                    http_client: ctx.data.read().await.get::<HttpKey>().cloned().unwrap(),
                    database: poise_pool,
                })
            })
        })
        .options(options)
        .build();

    Client::builder(discord_token, intents)
        .event_handler(handler)
        .register_songbird()
        .framework(framework)
        .type_map_insert::<HttpKey>(HttpClient::new()) // shared HTTP client for YoutubeDl
        .await
        .expect("Error creating client")
}
