use adventus::app_builder;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let token = std::env::var("DISCORD_TOKEN").expect("'DISCORD_TOKEN' not found");

    let mut client = app_builder::build(token).await;

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
