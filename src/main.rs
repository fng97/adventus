use adventus::app;
use adventus::database::local_database_url;
use anyhow::anyhow;
use shuttle_runtime::SecretStore;

#[shuttle_runtime::main]
async fn main(
    #[shuttle_runtime::Secrets] secret_store: SecretStore,
    #[shuttle_shared_db::Postgres(
        local_uri = &local_database_url("discord")
    )]
    pool: sqlx::PgPool,
) -> shuttle_serenity::ShuttleSerenity {
    let token = if let Some(token) = secret_store.get("DISCORD_TOKEN") {
        token
    } else {
        return Err(anyhow!("'DISCORD_TOKEN' was not found").into());
    };

    Ok(app::build(token, pool).await.into())
}
