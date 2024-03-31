use crate::common::{Context, Error};
use crate::introductions::queries::{clear_url_for_user_and_guild, set_url_for_user_and_guild};
use songbird::input::{Compose, YoutubeDl};
use std::time::Duration;

const YOUTUBE_URL_REGEX: &str =
    r"^(https?\:\/\/)?(www\.)?(youtube\.com\/watch\?v=|youtu\.be\/)[a-zA-Z0-9_-]{11}(\?[\w=&]*)?$";

async fn err_say(ctx: &Context<'_>, message: &str) -> Result<(), Error> {
    ctx.say(&format!("🔥 {message}")).await?;
    Ok(())
}

fn youtube_url_is_valid(url: &str) -> Result<bool, regex::Error> {
    let regex = regex::Regex::new(YOUTUBE_URL_REGEX)?;
    Ok(regex.is_match(url))
}

async fn get_yt_track_duration(
    http_client: reqwest::Client,
    yt_url: &str,
) -> Option<std::time::Duration> {
    let mut src = YoutubeDl::new(http_client, yt_url.to_string());

    match src.aux_metadata().await {
        Ok(metadata) => metadata.duration,
        Err(_) => None,
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
    const MAX_INTRO_DURATION: Duration = Duration::from_secs(5);

    let user_id = ctx.author().id;
    let guild_id = match ctx.guild_id() {
        Some(guild_id) => guild_id,
        None => {
            err_say(&ctx, "This command can only be used in a server.").await?;
            return Ok(());
        }
    };
    if !youtube_url_is_valid(url.as_str())? {
        err_say(&ctx, "Invalid YouTube URL.").await?;
        return Ok(());
    }

    if let Some(duration) =
        get_yt_track_duration(ctx.data().http_client.clone(), url.as_str()).await
    {
        if duration > MAX_INTRO_DURATION {
            err_say(&ctx, "The video must be less than 5 seconds long.").await?;
            return Ok(());
        }
    } else {
        return Err("Failed to get video duration.".into());
    }

    set_url_for_user_and_guild(
        user_id.get(),
        guild_id.get(),
        url.as_str(),
        ctx.data().database.clone(),
    )
    .await?;

    ctx.say("📯 Your intro sound has been set!").await?;

    Ok(())
}

/// Clear your intro sound.
///
/// This will stop your intro sound from being played when you join a voice
/// channel. To set a new intro sound, use the `/set_intro` command.
#[poise::command(slash_command)]
pub async fn clear_intro(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = match ctx.guild_id() {
        Some(guild_id) => guild_id,
        None => {
            err_say(&ctx, "This command can only be used in a server.").await?;
            return Ok(());
        }
    };

    clear_url_for_user_and_guild(
        ctx.author().id.get(),
        guild_id.get(),
        ctx.data().database.clone(),
    )
    .await?;

    ctx.say("🧹 Your intro sound has been cleared!").await?;

    Ok(())
}
