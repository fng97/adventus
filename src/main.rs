use anyhow::anyhow;
use reqwest::Client as HttpClient;
use serenity::{
    async_trait,
    client::{Client, Context, EventHandler},
    model::{gateway::Ready, voice::VoiceState},
    prelude::{GatewayIntents, TypeMapKey},
};
use shuttle_secrets::SecretStore;
use songbird::events::{Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent};
use songbird::input::YoutubeDl;
use songbird::SerenityInit;
use tracing::info;

const SONG_URL: &str = "https://www.youtube.com/watch?v=V66PMeImkxI";

struct HttpKey;

impl TypeMapKey for HttpKey {
    type Value = HttpClient;
}
struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }

    async fn voice_state_update(&self, ctx: Context, _old: Option<VoiceState>, new: VoiceState) {
        // Early return if there's no guild_id
        let guild_id = match new.guild_id {
            Some(guild_id) => guild_id,
            None => return,
        };

        // Early return if the user joining the channel is the bot itself
        if new.user_id == ctx.cache.current_user().id {
            info!("Bot joined a channel");
            return;
        }

        // Early return if there's no channel_id
        let channel_id = match new.channel_id {
            Some(channel_id) => channel_id,
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

        // Play
        let src = YoutubeDl::new(http_client, SONG_URL.to_string());
        let _ = handler.play_input(src.clone().into());
    }
}

#[shuttle_runtime::main]
async fn main(
    #[shuttle_secrets::Secrets] secret_store: SecretStore,
) -> shuttle_serenity::ShuttleSerenity {
    // Get the discord token set in `Secrets.toml`
    let token = if let Some(token) = secret_store.get("DISCORD_TOKEN") {
        token
    } else {
        return Err(anyhow!("'DISCORD_TOKEN' was not found").into());
    };

    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_VOICE_STATES;

    let client = Client::builder(&token, intents)
        .event_handler(Handler)
        .register_songbird()
        .type_map_insert::<HttpKey>(HttpClient::new()) // shared HTTP client for YoutubeDl
        .await
        .expect("Err creating client");

    Ok(client.into())
}

struct TrackErrorNotifier;

#[async_trait]
impl VoiceEventHandler for TrackErrorNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            for (state, handle) in *track_list {
                println!(
                    "Track {:?} encountered an error: {:?}",
                    handle.uuid(),
                    state.playing
                );
            }
        }

        None
    }
}
