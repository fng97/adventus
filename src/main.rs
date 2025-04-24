use adventus::{introductions, rolls, Data, Error};

use poise::serenity_prelude as serenity;
use songbird::SerenityInit;

async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot, .. } => {
            tracing::info!("Logged in as {}", data_about_bot.user.name);
        }
        serenity::FullEvent::VoiceStateUpdate { old, new, .. } => {
            introductions::handlers::voice_state_update(ctx, data, old, new).await?
        }
        _ => {}
    }
    Ok(())
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            tracing::warn!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                tracing::error!("Error while handling error: {:?}", e)
            }
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let token =
        std::env::var("DISCORD_TOKEN").expect("'DISCORD_TOKEN' environment variable not found");
    let intros_dir = std::env::var("INTROS_DIR")
        .map(std::path::PathBuf::from)
        .expect("'INTROS_DIR' environment variable not found");

    std::fs::create_dir_all(&intros_dir).expect("Failed to create intros dir"); // ensure intros dir exists

    let config = adventus::Config { intros_dir };

    let intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::MESSAGE_CONTENT
        | serenity::GatewayIntents::GUILD_VOICE_STATES;

    let framework = poise::Framework::builder()
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data { config })
            })
        })
        .options(poise::FrameworkOptions {
            commands: vec![
                introductions::commands::set_intro(),
                introductions::commands::clear_intro(),
                rolls::commands::roll(),
            ],
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            on_error: |error| Box::pin(on_error(error)),
            ..Default::default()
        })
        .build();

    let mut client = serenity::client::Client::builder(token, intents)
        .register_songbird()
        .framework(framework)
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
