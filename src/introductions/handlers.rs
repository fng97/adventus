use crate::{introductions, Data, Error};

/// Narrows VoiceStateUpdate event to user joining a voice channel.
fn user_joined_voice(
    ctx: &serenity::client::Context,
    old: &Option<serenity::all::VoiceState>,
    new: &serenity::all::VoiceState,
) -> Option<(
    serenity::all::ChannelId,
    serenity::all::GuildId,
    serenity::all::UserId,
)> {
    if new.user_id == ctx.cache.current_user().id {
        tracing::debug!("Bot joined the channel. Ignoring.");
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
        tracing::debug!("State change within same channel. Ignoring.");
        return None;
    }

    Some((channel_id, guild_id, new.user_id))
}

pub async fn voice_state_update(
    ctx: &serenity::client::Context,
    data: &Data,
    old: &Option<serenity::all::VoiceState>,
    new: &serenity::all::VoiceState,
) -> Result<(), Error> {
    let (channel_id, guild_id, user_id) = match user_joined_voice(ctx, old, new) {
        Some(values) => values,
        None => {
            tracing::debug!("User did not join a voice channel.");
            return Ok(());
        }
    };

    let intro_path = std::path::Path::new(data.config.intros_dir.as_path())
        .join(format!("{}_{}.opus", guild_id, user_id));
    if !intro_path.exists() {
        tracing::debug!("Intro sound not found for user {:?}.", user_id);
        return Ok(());
    }

    introductions::voice::play(ctx, guild_id, channel_id, intro_path.to_str().unwrap()).await;

    Ok(())
}
