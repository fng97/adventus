use crate::intro::play_intro;
use serenity::{
    all::{ChannelId, GuildId},
    async_trait,
    client::{Context, EventHandler},
    model::{gateway::Ready, id::UserId, voice::VoiceState},
};
use sqlx::PgPool;
use tracing::{debug, info};

pub struct Handler {
    pub database: PgPool,
}

fn user_joined_channel(
    ctx: &Context,
    old: Option<VoiceState>,
    new: VoiceState,
) -> Option<(ChannelId, GuildId, UserId)> {
    let user_id = new.user_id;

    if user_id == ctx.cache.current_user().id {
        debug!("Bot joined the channel. Ignoring.");
        return None;
    }

    let guild_id = match new.guild_id {
        Some(guild_id) => guild_id,
        None => {
            debug!("Non-guild voice state update received. Ignoring.");
            return None;
        }
    };

    let channel_id = match new.channel_id {
        Some(channel_id) => channel_id,
        None => {
            debug!("User left the channel. Ignoring.");
            return None;
        }
    };

    if let Some(old) = old {
        if let Some(old_channel_id) = old.channel_id {
            if old_channel_id == channel_id {
                debug!("State change within same channel. Ignoring.");
                return None;
            }
        }
    }

    Some((channel_id, guild_id, user_id))
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        if let Some((channel_id, guild_id, user_id)) = user_joined_channel(&ctx, old, new) {
            play_intro(&ctx, guild_id, channel_id, user_id, &self.database).await;
        }
    }
}
