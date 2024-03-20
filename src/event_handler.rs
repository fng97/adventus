use reqwest::Client as HttpClient;
use serenity::{
    all::{ChannelId, GuildId},
    async_trait,
    client::{Context, EventHandler},
    model::{gateway::Ready, id::UserId, voice::VoiceState},
    prelude::TypeMapKey,
};
use songbird::input::YoutubeDl;
use songbird::{events::TrackEvent, input::Compose};
use tracing::{debug, info};

use crate::voice_handler::TrackErrorNotifier;

const SONG_URL: &str = "https://www.youtube.com/watch?v=V66PMeImkxI";
const MAX_TRACK_DURATION: std::time::Duration = std::time::Duration::from_secs(5);

pub struct HttpKey;

impl TypeMapKey for HttpKey {
    type Value = HttpClient;
}
pub struct Handler;

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
        let (channel_id, guild_id, _user) = match user_joined_channel(&ctx, old, new) {
            Some((channel_id, guild_id, user)) => (channel_id, guild_id, user),
            None => return,
        };

        // Proceed with joining the channel and setting up the environment
        let manager = songbird::get(&ctx)
            .await
            .expect("Songbird Voice client placed in at initialisation.")
            .clone();

        // Attempt to join the voice channel, early return on failure
        let handler_lock = match manager.join(guild_id, channel_id).await {
            Ok(handler_lock) => handler_lock,
            Err(_) => return,
        };

        let mut handler: tokio::sync::MutexGuard<'_, songbird::Call> = handler_lock.lock().await;

        // Attach an event handler to see notifications of all track errors.
        handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);

        // Get the HTTP client, required by YoutubeDl
        let http_client = {
            let data = ctx.data.read().await;
            data.get::<HttpKey>()
                .cloned()
                .expect("Guaranteed to exist in the typemap.")
        };

        // get source from URL
        let src = YoutubeDl::new(http_client, SONG_URL.to_string());

        // check the track is less than the limit
        let duration = match src.clone().aux_metadata().await.unwrap().duration {
            Some(duration) => duration,
            None => {
                info!("Track duration is unknown");
                return;
            }
        };

        // FIXME: We should check this before we join the channel
        if duration > MAX_TRACK_DURATION {
            info!("Track duration is too long: {:?}", duration);
            return;
        }

        // play the source
        // TODO: try play_only_input instead
        let _ = handler.play_input(src.clone().into());
    }
}
