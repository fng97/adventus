use crate::common::Data;
use crate::handlers;
use crate::introductions;
use crate::rolls;

use serenity::{client::Client, prelude::GatewayIntents};
use songbird::SerenityInit;

pub async fn build(discord_token: String) -> Client {
    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_VOICE_STATES;

    let framework = poise::Framework::builder()
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        .options(poise::FrameworkOptions {
            commands: vec![
                introductions::commands::set_intro(),
                introductions::commands::clear_intro(),
                rolls::commands::roll(),
            ],
            event_handler: |ctx, event, framework, data| {
                Box::pin(handlers::event_handler(ctx, event, framework, data))
            },
            on_error: |error| Box::pin(handlers::on_error(error)),
            ..Default::default()
        })
        .build();

    Client::builder(discord_token, intents)
        .register_songbird()
        .framework(framework)
        .await
        .expect("Error creating client")
}
