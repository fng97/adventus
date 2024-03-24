use crate::database::migrate;
use crate::intro::{get_url_for_user_and_guild, user_joined_voice};
use crate::player::get_yt_track_duration;
use crate::player::play;

use poise::serenity_prelude as serenity;
use serenity::{client::Client, prelude::GatewayIntents};
use songbird::SerenityInit;
use sqlx::PgPool;
use std::time::Duration;
use tracing::{error, info, warn};
use url::Url;

struct Data {
    pub http_client: reqwest::Client,
    pub database: sqlx::PgPool,
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// Set your intro sound from a YouTube URL.
///
/// This sound plays when you join a voice channel. The sound is streamed
/// directly from YouTube. The link you provide must be to a YouTube video that
/// is less than 5 seconds long.
#[poise::command(slash_command)]
pub async fn set_intro(
    ctx: Context<'_>,
    #[description = "YouTube URL (video must be less than 5s long)"] url: String,
) -> Result<(), Error> {
    // vaidate url
    let url = match Url::parse(&url) {
        Ok(url) => url,
        Err(_) => {
            ctx.say("Invalid URL.").await?;
            return Ok(());
        }
    };

    // validate youtube URL

    if let Some(duration) = get_yt_track_duration(&ctx.data().http_client, url.as_str()).await {
        if duration > Duration::from_secs(5) {
            ctx.say("The video must be less than 5 seconds long.")
                .await?;
            return Ok(());
        }
    } else {
        return Err("Failed to get video duration.".into());
    }

    let user_id = ctx.author().id;
    let guild_id = match ctx.guild_id() {
        Some(guild_id) => guild_id,
        None => {
            ctx.say("This command can only be used in a server.")
                .await?;
            return Ok(());
        }
    };

    sqlx::query!(
        r#"
        INSERT INTO intros (user_snowflake, guild_snowflake, yt_url)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_snowflake, guild_snowflake)
        DO UPDATE SET yt_url = EXCLUDED.yt_url
        "#,
        user_id.get() as i64,
        guild_id.get() as i64,
        url.as_str(),
    )
    .execute(&ctx.data().database)
    .await
    .unwrap();

    ctx.say("Your intro sound has been set!").await?;

    Ok(())
}

/// Clear your intro sound.
///
/// This will stop your intro sound from being played when you join a voice
/// channel. To set a new intro sound, use the `/set_intro` command.
#[poise::command(slash_command)]
pub async fn clear_intro(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id;
    let guild_id = match ctx.guild_id() {
        Some(guild_id) => guild_id,
        None => {
            ctx.say("This command can only be used in a server.")
                .await?;
            return Ok(());
        }
    };

    match sqlx::query!(
        r#"
        DELETE FROM intros
        WHERE user_snowflake = $1 AND guild_snowflake = $2
        "#,
        user_id.get() as i64,
        guild_id.get() as i64,
    )
    .execute(&ctx.data().database)
    .await
    {
        Ok(_) => {}
        Err(sqlx::Error::RowNotFound) => {
            ctx.say("You don't have an intro sound set.").await?;
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    }

    ctx.say("Your intro sound has been cleared!").await?;

    Ok(())
}

async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot, .. } => {
            info!("Logged in as {}", data_about_bot.user.name);
        }
        serenity::FullEvent::VoiceStateUpdate { old, new, .. } => {
            if let Some((channel_id, guild_id, user_id)) = user_joined_voice(ctx, old, new) {
                if let Some(url) =
                    get_url_for_user_and_guild(user_id.get(), guild_id.get(), &data.database).await
                {
                    play(ctx, guild_id, channel_id, &url, &data.http_client).await;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            warn!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                error!("Error while handling error: {:?}", e)
            }
        }
    }
}

pub async fn build(discord_token: String, pool: PgPool) -> Client {
    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_VOICE_STATES;

    migrate(&pool).await;

    let framework = poise::Framework::builder()
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    http_client: reqwest::Client::new(),
                    database: pool,
                })
            })
        })
        .options(poise::FrameworkOptions {
            commands: vec![set_intro(), clear_intro()],
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            on_error: |error| Box::pin(on_error(error)),
            ..Default::default()
        })
        .build();

    Client::builder(discord_token, intents)
        .register_songbird()
        .framework(framework)
        .await
        .expect("Error creating client")
}
