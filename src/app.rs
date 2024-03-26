use crate::database_setup::migrate;
use crate::introductions::queries::{get_url_for_user_and_guild, set_url_for_user_and_guild};
use crate::introductions::voice::get_yt_track_duration;
use crate::introductions::voice::play;

use ::serenity::all::Mentionable;
use poise::serenity_prelude as serenity;
use rand::Rng;
use serenity::all::{ChannelId, GuildId, UserId, VoiceState};
use serenity::{client::Client, prelude::GatewayIntents};
use songbird::SerenityInit;
use sqlx::PgPool;
use std::time::Duration;
use tracing::debug;
use tracing::{error, info, warn};
use url::Url; // For rolling logic // Importing necessary trait for choice parameters

fn user_joined_voice(
    ctx: &serenity::client::Context,
    old: &Option<VoiceState>,
    new: &VoiceState,
) -> Option<(ChannelId, GuildId, UserId)> {
    if new.user_id == ctx.cache.current_user().id {
        debug!("Bot joined the channel. Ignoring.");
        return None;
    }

    let guild_id = new.guild_id?;
    let channel_id = new.channel_id?;

    if old
        .as_ref()
        .and_then(|o| o.channel_id)
        .map(|old_channel_id| old_channel_id == channel_id)
        .unwrap_or(false)
    {
        debug!("State change within same channel. Ignoring.");
        return None;
    }

    Some((channel_id, guild_id, new.user_id))
}

struct Data {
    pub http_client: reqwest::Client,
    pub database: sqlx::PgPool,
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// Roll the dice!
#[poise::command(slash_command)]
pub async fn roll(
    ctx: Context<'_>,
    #[description = "Number of sides of the dice."]
    #[min = 2]
    #[max = 100]
    sides: u8,
    #[description = "Number of dice to roll."]
    #[min = 1]
    #[max = 10]
    rolls: Option<u8>,
) -> Result<(), Error> {
    const DEFAULT_NUM_ROLLS: u8 = 1;
    let rolls = rolls.unwrap_or(DEFAULT_NUM_ROLLS);

    let results: Vec<u8> = (0..rolls)
        .map(|_| rand::thread_rng().gen_range(1..sides))
        .collect();

    // Create a string of the results to send in the message
    let results_str = results
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(", ");

    // Send the message
    ctx.say(format!(
        "{} rolled {}.",
        ctx.author().mention(),
        results_str
    ))
    .await?;

    Ok(())
}

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

    set_url_for_user_and_guild(
        user_id.get(),
        guild_id.get(),
        url.as_str(),
        &ctx.data().database,
    )
    .await?;

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
            commands: vec![set_intro(), clear_intro(), roll()],
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
