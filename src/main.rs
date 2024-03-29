use adventus::app_builder;
use adventus::database_setup::local_database_url;
use shuttle_runtime::SecretStore;

#[shuttle_runtime::main]
async fn main(
    #[shuttle_runtime::Secrets] secret_store: SecretStore,
    #[shuttle_shared_db::Postgres(
        local_uri = &local_database_url("discord")
    )]
    pool: sqlx::PgPool,
) -> shuttle_serenity::ShuttleSerenity {
    let token = secret_store
        .get("DISCORD_TOKEN")
        .expect("'DISCORD_TOKEN' not found");

    Ok(app_builder::build(token, pool).await.into())
}
