#[cfg(test)]
mod tests {
    use chrono::Utc;
    use sqlx::{Connection, PgConnection};
    use std::env;
    use uuid::Uuid;

    pub struct DatabaseSettings {
        pub username: String,
        pub password: String,
        pub port: u16,
        pub host: String,
        pub database_name: String,
    }

    impl DatabaseSettings {
        pub fn connection_string(&self) -> String {
            format!(
                "postgres://{}:{}@{}:{}/{}",
                self.username, self.password, self.host, self.port, self.database_name
            )
        }
    }

    #[tokio::test]
    async fn test_test() {
        let database_settings: DatabaseSettings = DatabaseSettings {
            username: "postgres".to_string(),
            password: "password".to_string(),
            port: 5432,
            host: env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string()),
            database_name: "discord".to_string(),
        };

        let connection_string = database_settings.connection_string();

        let mut connection = PgConnection::connect(&connection_string)
            .await
            .expect("Failed to connect to Postgres.");

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
        .execute(&mut connection)
        .await
        .unwrap();

        // get record from database

        let record = sqlx::query!(
            r#"
            SELECT user_snowflake, guild_snowflake, yt_url
            FROM intros
            WHERE user_snowflake = $1 AND guild_snowflake = $2
            "#,
            USER_SNOWFLAKE,
            GUILD_SNOWFLAKE
        )
        .fetch_one(&mut connection)
        .await
        .unwrap();

        // assertions
        assert_eq!(record.user_snowflake, USER_SNOWFLAKE);
        assert_eq!(record.guild_snowflake, GUILD_SNOWFLAKE);
        assert_eq!(record.yt_url, URL);
    }
}
