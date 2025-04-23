use once_cell::sync::Lazy;
use serenity::{
    all::{ChannelId, GuildId},
    async_trait,
    client::Context,
};
use songbird::{
    events::{Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent},
    Songbird,
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{
    sync::{Mutex, MutexGuard},
    task::JoinHandle,
};
use tracing::{info, warn};

static DISCONNECT_HANDLES: Lazy<Mutex<HashMap<GuildId, JoinHandle<()>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn play(ctx: &Context, guild_id: GuildId, channel_id: ChannelId, file_path: &str) {
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let handler_lock = match manager.join(guild_id, channel_id).await {
        Ok(handler_lock) => handler_lock,
        Err(e) => {
            warn!("Failed to join channel: {:?}", e);
            return;
        }
    };

    let mut handler: MutexGuard<'_, songbird::Call> = handler_lock.lock().await;

    handler.add_global_event(TrackEvent::Error.into(), TrackErrorHandler);

    // Read bytes from the provided file and play them
    let file_bytes = tokio::fs::read(file_path)
        .await
        .expect("Failed to read file");
    // let input = File::from_memory(file_bytes).create_async();

    let _ = handler.play_input(file_bytes.into());

    schedule_disconnect(guild_id, manager).await;
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

async fn schedule_disconnect(guild_id: GuildId, songbird: Arc<Songbird>) {
    const DISCONNECT_AFTER: Duration = Duration::from_secs(60 * 5);

    let mut handles = DISCONNECT_HANDLES.lock().await;

    // cancel any existing disconnect future for this guild
    if let Some(handle) = handles.get(&guild_id) {
        handle.abort();
        handles.remove(&guild_id);
    }

    let handle = tokio::spawn(async move {
        tokio::time::sleep(DISCONNECT_AFTER).await;
        if songbird.leave(guild_id).await.is_ok() {
            info!(
                "Disconnected from guild {} after {} minutes without using voice.",
                guild_id,
                DISCONNECT_AFTER.as_secs() / 60
            );
        }
        let mut handles = DISCONNECT_HANDLES.lock().await;
        handles.remove(&guild_id);
    });

    handles.insert(guild_id, handle);
}
