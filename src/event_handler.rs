use reqwest::Client as HttpClient;
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{gateway::Ready, voice::VoiceState},
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

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        // Early return if there's no guild_id
        let guild_id = match new.guild_id {
            Some(guild_id) => guild_id,
            None => {
                debug!("Non-guild voice state update received. Ignoring.");
                return;
            }
        };

        // Early return if the user joining the channel is the bot itself
        if new.user_id == ctx.cache.current_user().id {
            debug!("State update is for the bot itself. Ignoring.");
            return;
        }

        // Early return if there's no new channel_id
        let channel_id = match new.channel_id {
            Some(channel_id) => channel_id,
            None => {
                debug!("User left the channel. Ignoring.");
                return;
            }
        };

        if let Some(old) = old {
            if let Some(old_channel_id) = old.channel_id {
                if old_channel_id == channel_id {
                    debug!("State change within same channel. Ignoring.");
                    return;
                }
            }
        }

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
        let _ = handler.play_input(src.clone().into());
    }
}
