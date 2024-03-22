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
