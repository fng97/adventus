use crate::common::{Data, Error};
use crate::introductions::queries::get_url_for_user_and_guild;
use crate::introductions::voice::play;

use poise::serenity_prelude as serenity;
use serenity::all::{ChannelId, GuildId, UserId, VoiceState};
use tracing::{debug, error, info, warn};

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

pub async fn event_handler(
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

pub async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
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
