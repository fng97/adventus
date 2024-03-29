use crate::common::{Data, Error};
use crate::introductions::queries::get_url_for_user_and_guild;
use crate::introductions::voice::play;

use serenity::{
    all::{ChannelId, GuildId, UserId, VoiceState},
    client::Context,
};
use tracing::debug;

/// Narrows VoiceStateUpdate event to user joining a voice channel.
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

pub async fn voice_state_update(
    ctx: &Context,
    data: &Data,
    old: &Option<VoiceState>,
    new: &VoiceState,
) -> Result<(), Error> {
    let (channel_id, guild_id, user_id) = match user_joined_voice(ctx, old, new) {
        Some(values) => values,
        None => {
            debug!("User did not join a voice channel.");
            return Ok(());
        }
    };

    let url = match get_url_for_user_and_guild(user_id.get(), guild_id.get(), &data.database).await
    {
        Some(url) => url,
        None => {
            debug!(
                "No introduction found for user {} in guild {}",
                user_id, guild_id
            );
            return Ok(());
        }
    };

    play(
        ctx,
        guild_id,
        channel_id,
        &url,
        &data.http_client,
        &data.last_active,
    )
    .await;

    Ok(())
}
