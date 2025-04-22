use crate::common::{Context, Error};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

const INTRO_DIR: &str = "./intros";

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
    const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB
    const MAX_DURATION: Duration = Duration::from_secs(5);

    fs::create_dir_all(INTRO_DIR)?; // ensure intros dir exists

    let user_id = ctx.author().id;
    let guild_id = match ctx.guild_id() {
        Some(guild_id) => guild_id,
        None => {
            err_say(&ctx, "This command can only be used in a server.").await?;
            return Ok(());
        }
    };

    if u64::from(attachment.size) > MAX_FILE_SIZE {
        err_say(&ctx, "File size exceeds the 10MB limit.").await?; // default attachment size limit
        return Ok(());
    }

    let attachment_path = Path::new(INTRO_DIR).join(format!("{}_{}_temp", guild_id, user_id));
    let new_intro_path = Path::new(INTRO_DIR).join(format!("{}_{}_new.opus", guild_id, user_id));
    let final_intro_path = Path::new(INTRO_DIR).join(format!("{}_{}.opus", guild_id, user_id));

    let file_bytes = attachment.download().await?;
    fs::write(&attachment_path, &file_bytes)?;

    let output = Command::new("ffmpeg")
        .args(&[
            "-y",                                       // overwrite without asking
            "-i",                                       // input ↓
            attachment_path.to_str().unwrap(),          // input file
            "-t",                                       // trim to duration ↓
            &format!("{}", MAX_DURATION.as_secs_f64()), // MAX_DURATION seconds
            "-vn",                                      // drop any video streams
            "-c:a",                                     // audio codec to use ↓
            "libopus",                                  // opus
            "-b:a",                                     // audio bitrate ↓
            "16k",                                      // ~16 kbps
            "-ac",                                      // audio channel ↓
            "1",                                        // mono
            "-ar",                                      // audio sample rate ↓
            "16000",                                    // 16 kHz
            "-application",                             // application type ↓
            "voip",                                     // tune for VoIP/voice chat
            "-vbr",                                     // variable bit rate ↓
            "constrained",                              // constrained
            new_intro_path.to_str().unwrap(),           // output file
        ])
        .output()?;

    if !output.status.success() {
        fs::remove_file(&attachment_path).ok();
        fs::remove_file(&new_intro_path).ok();
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!("ffmpeg failed with status {}: {}", output.status, stderr);
        err_say(&ctx, "Failed to process audio.").await?;
        return Ok(());
    }

    fs::rename(&new_intro_path, &final_intro_path)?;
    fs::remove_file(&attachment_path)?;

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
