static DISCONNECT_HANDLES: once_cell::sync::Lazy<
    tokio::sync::Mutex<
        std::collections::HashMap<serenity::all::GuildId, tokio::task::JoinHandle<()>>,
    >,
> = once_cell::sync::Lazy::new(|| tokio::sync::Mutex::new(std::collections::HashMap::new()));

pub async fn play(
    ctx: &serenity::client::Context,
    guild_id: serenity::all::GuildId,
    channel_id: serenity::all::ChannelId,
    file_path: &str,
) {
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let handler_lock = match manager.join(guild_id, channel_id).await {
        Ok(handler_lock) => handler_lock,
        Err(e) => {
            tracing::warn!("Failed to join channel: {:?}", e);
            return;
        }
    };

    let mut handler: tokio::sync::MutexGuard<'_, songbird::Call> = handler_lock.lock().await;

    handler.add_global_event(
        songbird::events::TrackEvent::Error.into(),
        TrackErrorHandler,
    );

    let file_bytes = tokio::fs::read(file_path)
        .await
        .expect("Failed to read file");

    let _ = handler.play_input(file_bytes.into());

    schedule_disconnect(guild_id, manager).await;
}

struct TrackErrorHandler;

#[serenity::async_trait]
impl songbird::events::EventHandler for TrackErrorHandler {
    async fn act(
        &self,
        ctx: &songbird::events::EventContext<'_>,
    ) -> Option<songbird::events::Event> {
        if let songbird::events::EventContext::Track(track_list) = ctx {
            for (state, handle) in *track_list {
                tracing::warn!(
                    "Track {:?} encountered an error: {:?}",
                    handle.uuid(),
                    state.playing
                );
            }
        }

        None
    }
}

async fn schedule_disconnect(
    guild_id: serenity::all::GuildId,
    songbird: std::sync::Arc<songbird::Songbird>,
) {
    const DISCONNECT_AFTER: std::time::Duration = std::time::Duration::from_secs(60 * 5);

    let mut handles = DISCONNECT_HANDLES.lock().await;

    // cancel any existing disconnect future for this guild
    if let Some(handle) = handles.get(&guild_id) {
        handle.abort();
        handles.remove(&guild_id);
    }

    let handle = tokio::spawn(async move {
        tokio::time::sleep(DISCONNECT_AFTER).await;
        if songbird.leave(guild_id).await.is_ok() {
            tracing::info!(
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
