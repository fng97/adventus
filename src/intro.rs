#[cfg(test)]
mod tests {
    use chrono::Utc;
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
    async fn test_test() {
        let connection_pool = get_test_database_pgpool().await;

        // add user, guild, and url to database

        const USER_SNOWFLAKE: i64 = 123456789123456789;
        const GUILD_SNOWFLAKE: i64 = 987654321987654321;
        const URL: &str = "https://example.com";

        sqlx::query!(
            r#"
            INSERT INTO intros (user_snowflake, guild_snowflake, yt_url)
            VALUES ($1, $2, $3)
            "#,
            USER_SNOWFLAKE,
            GUILD_SNOWFLAKE,
            URL
        )
        .execute(&connection_pool)
        .await
        .unwrap();

        // get record from database

        let record = sqlx::query!(
            r#"
            SELECT user_snowflake, guild_snowflake, yt_url, set_at
            FROM intros
            WHERE user_snowflake = $1 AND guild_snowflake = $2
            "#,
            USER_SNOWFLAKE,
            GUILD_SNOWFLAKE
        )
        .fetch_one(&connection_pool)
        .await
        .unwrap();

        // assertions
        assert_eq!(record.user_snowflake, USER_SNOWFLAKE);
        assert_eq!(record.guild_snowflake, GUILD_SNOWFLAKE);
        assert_eq!(record.yt_url, URL);
        assert_ne!(record.set_at, Utc::now());
    }

    #[tokio::test]
    async fn test_test2() {
        let connection_pool = get_test_database_pgpool().await;

        // add user, guild, and url to database

        const USER_SNOWFLAKE: i64 = 123456789123456789;
        const GUILD_SNOWFLAKE: i64 = 987654321987654321;
        const URL: &str = "https://example.com";

        sqlx::query!(
            r#"
            INSERT INTO intros (user_snowflake, guild_snowflake, yt_url)
            VALUES ($1, $2, $3)
            "#,
            USER_SNOWFLAKE,
            GUILD_SNOWFLAKE,
            URL
        )
        .execute(&connection_pool)
        .await
        .unwrap();

        // get record from database

        let record = sqlx::query!(
            r#"
            SELECT user_snowflake, guild_snowflake, yt_url, set_at
            FROM intros
            WHERE user_snowflake = $1 AND guild_snowflake = $2
            "#,
            USER_SNOWFLAKE,
            GUILD_SNOWFLAKE
        )
        .fetch_one(&connection_pool)
        .await
        .unwrap();

        // assertions
        assert_eq!(record.user_snowflake, USER_SNOWFLAKE);
        assert_eq!(record.guild_snowflake, GUILD_SNOWFLAKE);
        assert_eq!(record.yt_url, URL);
        assert_ne!(record.set_at, Utc::now());
    }
}
