use crate::common::{Context, Error};
use crate::introductions::queries::set_url_for_user_and_guild;
use crate::introductions::voice::get_yt_track_duration;

use std::time::Duration;

const YOUTUBE_URL_REGEX: &str = r"^((?:https?:)?\/\/)?((?:www|m)\.)?((?:youtube(-nocookie)?\.com|youtu.be))(\/(?:[\w\-]+\?v=|embed\/|live\/|v\/)?)([\w\-]+)(\S+)?$";

fn youtube_url_is_valid(url: &str) -> Result<bool, regex::Error> {
    let regex = regex::Regex::new(YOUTUBE_URL_REGEX)?;
    Ok(regex.is_match(url))
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
    if !youtube_url_is_valid(url.as_str())? {
        ctx.say("Invalid YouTube URL.").await?;
        return Ok(());
    }

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
        Err(e) => return Err(e.into()),
    }

    ctx.say("Your intro sound has been cleared!").await?;

    Ok(())
}
