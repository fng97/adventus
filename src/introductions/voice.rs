use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use serenity::{
    all::{ChannelId, GuildId},
    async_trait,
    client::Context,
};
use songbird::input::YoutubeDl;
use songbird::{
    events::{Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent},
    Songbird,
};
use tokio::{sync::Mutex, time::interval};
use tracing::{info, warn};

// TODO: Add error handling
pub async fn play(
    ctx: &Context,
    guild_id: GuildId,
    channel_id: ChannelId,
    yt_url: &str,
    http_client: &reqwest::Client,
    last_active: &tokio::sync::Mutex<std::collections::HashMap<GuildId, std::time::Instant>>,
) {
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let handler_lock = match manager.join(guild_id, channel_id).await {
        Ok(handler_lock) => handler_lock,
        Err(_) => return,
    };

    let mut handler: tokio::sync::MutexGuard<'_, songbird::Call> = handler_lock.lock().await;

    handler.add_global_event(TrackEvent::Error.into(), TrackErrorHandler);

    let _ = handler.play_only_input(YoutubeDl::new(http_client.clone(), yt_url.to_string()).into());

    // update activity time - used for voice channel disconnect
    last_active
        .lock()
        .await
        .insert(guild_id, std::time::Instant::now());
}

struct TrackErrorHandler;

#[async_trait]
impl VoiceEventHandler for TrackErrorHandler {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            for (state, handle) in *track_list {
                warn!(
                    "Track {:?} encountered an error: {:?}",
                    handle.uuid(),
                    state.playing
                );
            }
        }

        None
    }
}

pub async fn disconnect_inactive_clients(
    songbird: Arc<Songbird>,
    last_active: Arc<Mutex<std::collections::HashMap<GuildId, Instant>>>,
) {
    const CHECK_EVERY: Duration = Duration::from_secs(2);
    const DISCONNECT_AFTER: Duration = Duration::from_secs(8);

    let mut interval = interval(CHECK_EVERY);

    loop {
        interval.tick().await;
        let now = Instant::now();

        let guilds_to_disconnect: Vec<GuildId> = {
            let last_active_guard = last_active.lock().await;
            last_active_guard
                .iter()
                .filter_map(|(&guild_id, &last_active_time)| {
                    if now.duration_since(last_active_time) > DISCONNECT_AFTER {
                        Some(guild_id)
                    } else {
                        None
                    }
                })
                .collect()
        };

        for guild_id in guilds_to_disconnect {
            if songbird.leave(guild_id).await.is_ok() {
                info!("Disconnected from guild {} due to inactivity", guild_id);
            }
            last_active.lock().await.remove(&guild_id);
        }
    }
}
