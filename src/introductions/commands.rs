use crate::common::{Context, Error};
use std::fs;
use std::fs::File;
use std::path::Path;
use std::time::Duration;
use symphonia::core::{
    formats::FormatOptions, io::MediaSourceStream, meta::MetadataOptions, probe::Hint,
};
use symphonia::default::get_probe;

const INTRO_DIR: &str = "./intros";
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB
const MAX_DURATION: Duration = Duration::from_secs(5);

async fn err_say(ctx: &Context<'_>, message: &str) -> Result<(), Error> {
    ctx.say(&format!("🔥 {message}")).await?;
    Ok(())
}

/// Attach an audio file to be set as your intro.
///
/// This sound plays when you join a voice channel. The file must be smaller than 10MB. The sound must be less than 5s
/// long.
#[poise::command(slash_command)]
pub async fn set_intro(
    ctx: Context<'_>,
    #[description = "Attach an audio file"] attachment: poise::serenity_prelude::Attachment,
) -> Result<(), Error> {
    fs::create_dir_all(INTRO_DIR)?; // ensure intros dir exists

    if u64::from(attachment.size) > MAX_FILE_SIZE {
        ctx.say("❌ File size exceeds the 10MB limit.").await?;
        return Ok(());
    }

    let file_extension = attachment.filename.split('.').last().unwrap_or("");
    if !["mp3"].contains(&file_extension.to_lowercase().as_str()) {
        ctx.say("❌ Unsupported file type. Please upload an MP3, WAV, or OGG file.")
            .await?;
        return Ok(());
    }

    let file_path = Path::new(INTRO_DIR).join(format!(
        "{}_{}.{}",
        ctx.guild_id().unwrap_or_default(),
        ctx.author().id,
        file_extension
    ));

    let file_bytes = attachment.download().await?;
    fs::write(&file_path, &file_bytes)?;

    if get_audio_duration(&file_path).await? > MAX_DURATION {
        fs::remove_file(&file_path)?;
        err_say(&ctx, "Audio duration exceeds the 5-second limit.").await?;
        return Ok(());
    }

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

    let user_intro_pattern = format!("{}_{}", guild_id, ctx.author().id);
    let intro_files = fs::read_dir(INTRO_DIR)?;

    let file_removed = intro_files
        .filter_map(Result::ok)
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(&user_intro_pattern)
        })
        .map(|entry| fs::remove_file(entry.path()).is_ok())
        .unwrap_or(false);

    if file_removed {
        ctx.say("🧹 Your intro sound has been cleared!").await?;
    } else {
        err_say(&ctx, "No intro sound found to clear.").await?;
    }

    Ok(())
}

async fn get_audio_duration(file_path: &Path) -> Result<Duration, Error> {
    let file = File::open(file_path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let hint = Hint::new();

    let format = get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?
        .format;

    if format.tracks().len() != 1 {
        return Err("The audio file must contain exactly one track.".into());
    }

    if let Some(track) = format.tracks().first() {
        if let Some(duration) = track.codec_params.n_frames.map(|frames| {
            let sample_rate = track.codec_params.sample_rate.unwrap_or(1);
            Duration::from_secs_f64(frames as f64 / sample_rate as f64)
        }) {
            return Ok(duration);
        }
    }

    Err("Unable to determine audio duration.".into())
}
