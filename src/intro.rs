use crate::player::play;
use serenity::{
    all::{ChannelId, GuildId, UserId},
    client::Context,
};
use sqlx::PgPool;
use tracing::{debug, info};

const _MAX_TRACK_DURATION: std::time::Duration = std::time::Duration::from_secs(5);

pub async fn get_url_for_user_and_guild(
    user_id: u64,
    guild_id: u64,
    pool: &PgPool,
) -> Option<String> {
    let user_id = user_id as i64;
    let guild_id = guild_id as i64;

    match sqlx::query!(
        r#"
        SELECT yt_url
        FROM intros
        WHERE user_snowflake = $1 AND guild_snowflake = $2
        "#,
        user_id,
        guild_id,
    )
    .fetch_one(pool)
    .await
    {
        Ok(record) => Some(record.yt_url),
        Err(_) => {
            debug!("No intro found for user {} in guild {}", user_id, guild_id);
            None
        }
    }
}

pub async fn play_intro(
    ctx: &Context,
    guild_id: GuildId,
    channel_id: ChannelId,
    user_id: UserId,
    pool: &PgPool,
) {
    if let Some(url) = get_url_for_user_and_guild(user_id.get(), guild_id.get(), pool).await {
        play(ctx, guild_id, channel_id, &url).await;
    } else {
        info!("No intro found for user {} in guild {}", user_id, guild_id);
    }
}

#[cfg(test)]
mod tests {
    use crate::intro::get_url_for_user_and_guild;
    use sqlx::{Connection, Executor, PgConnection, PgPool};
    use std::env;
    use uuid::Uuid;

    pub struct TestDatabaseSettings {
        pub username: String,
        pub password: String,
        pub port: u16,
        pub host: String,
        pub database_name: String,
    }

    impl TestDatabaseSettings {
        pub fn connection_string_without_db(&self) -> String {
            format!(
                "postgres://{}:{}@{}:{}",
                self.username, self.password, self.host, self.port
            )
        }

        pub fn connection_string(&self) -> String {
            format!(
                "{}/{}",
                self.connection_string_without_db(),
                self.database_name
            )
        }
    }

    pub async fn get_test_database_pgpool() -> PgPool {
        let test_database_settings = TestDatabaseSettings {
            username: "postgres".to_string(),
            password: "password".to_string(),
            port: 5432,
            host: env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string()),
            database_name: Uuid::new_v4().to_string(), // unique db for each test
        };

        configure_database(&test_database_settings).await
    }

    async fn configure_database(config: &TestDatabaseSettings) -> PgPool {
        // Create database
        let _ = PgConnection::connect(&config.connection_string_without_db())
            .await
            .expect("Failed to connect to Postgres")
            .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
            .await
            .expect("Failed to create database.");

        // Migrate database
        let connection_pool = PgPool::connect(&config.connection_string())
            .await
            .expect("Failed to connect to Postgres.");
        sqlx::migrate!("./migrations")
            .run(&connection_pool)
            .await
            .expect("Failed to migrate the database");

        connection_pool
    }

    #[tokio::test]
    async fn gets_url_for_user_and_guild() {
        let connection_pool = get_test_database_pgpool().await;

        // add user, guild, and url to database

        const USER_SNOWFLAKE: u64 = 123456789123456789;
        const GUILD_SNOWFLAKE: u64 = 987654321987654321;
        const URL: &str = "https://example.com";

        sqlx::query!(
            r#"
            INSERT INTO intros (user_snowflake, guild_snowflake, yt_url)
            VALUES ($1, $2, $3)
            "#,
            USER_SNOWFLAKE as i64,
            GUILD_SNOWFLAKE as i64,
            URL
        )
        .execute(&connection_pool)
        .await
        .unwrap();

        // get record from database

        let expected_url =
            get_url_for_user_and_guild(USER_SNOWFLAKE, GUILD_SNOWFLAKE, &connection_pool)
                .await
                .unwrap();

        // assertions
        assert_eq!(expected_url, URL);
    }
}
