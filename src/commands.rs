use crate::player::get_yt_track_duration;
use std::time::Duration;
use tracing::warn;
use url::Url;

pub struct Data {
    pub http_client: reqwest::Client,
    pub database: sqlx::PgPool,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

pub async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    // This is our custom error handler
    // They are many errors that can occur, so we only handle the ones we want to customize
    // and forward the rest to the default handler
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            warn!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                warn!("Error while handling error: {}", e)
            }
        }
    }
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