use crate::voice_handler::TrackErrorNotifier;
use reqwest::Client as HttpClient;
use serenity::{
    all::{ChannelId, GuildId},
    client::Context,
    prelude::TypeMapKey,
};
use songbird::input::YoutubeDl;
use songbird::{events::TrackEvent, input::Compose};
use tracing::info;

pub struct HttpKey;

impl TypeMapKey for HttpKey {
    type Value = HttpClient;
}

pub async fn play(ctx: &Context, guild_id: GuildId, channel_id: ChannelId, yt_url: &str) {
    // Proceed with joining the channel and setting up the environment
    let manager = songbird::get(ctx)
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
    let src = YoutubeDl::new(http_client, yt_url.to_string());

    // play the source
    // TODO: try play_only_input instead
    let _ = handler.play_input(src.clone().into());
}

pub async fn get_yt_track_duration(ctx: &Context, yt_url: &str) -> Option<std::time::Duration> {
    let http_client = {
        let data = ctx.data.read().await;
        data.get::<HttpKey>()
            .cloned()
            .expect("Guaranteed to exist in the typemap.")
    };

    let src = YoutubeDl::new(http_client, yt_url.to_string());

    match src.clone().aux_metadata().await.unwrap().duration {
        Some(duration) => Some(duration),
        None => {
            info!("Track duration is unknown");
            None
        }
    }
}
