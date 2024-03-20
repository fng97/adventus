#[cfg(test)]
mod tests {
    use crate::configuration::get_configuration;
    use chrono::Utc;
    use sqlx::{Connection, PgConnection};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_test() {
        let configuration = get_configuration().expect("Failed to read configuration.");
        let connection_string = configuration.database.connection_string();

        let mut connection = PgConnection::connect(&connection_string)
            .await
            .expect("Failed to connect to Postgres.");

        // add user, guild, and url to database

        const USER_SNOWFLAKE: i64 = 123456789123456789;
        const GUILD_SNOWFLAKE: i64 = 987654321987654321;

        sqlx::query!(
            r#"
            INSERT INTO users (id, snowflake, registered_at)
            VALUES ($1, $2, $3)
            "#,
            Uuid::new_v4(),
            USER_SNOWFLAKE,
            Utc::now()
        )
        .execute(&mut connection)
        .await
        .unwrap();

        sqlx::query!(
            r#"
            INSERT INTO guilds (id, snowflake, registered_at)
            VALUES ($1, $2, $3)
            "#,
            Uuid::new_v4(),
            GUILD_SNOWFLAKE,
            Utc::now()
        )
        .execute(&mut connection)
        .await
        .unwrap();

        sqlx::query!(
            r#"
            INSERT INTO users_guilds_urls (id, user_id, guild_id, url, created_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            Uuid::new_v4(),
            USER_SNOWFLAKE,
            GUILD_SNOWFLAKE,
            "https://example.com",
            Utc::now()
        )
        .execute(&mut connection)
        .await
        .unwrap();

        assert_eq!(configuration.database.username, "postgres");
    }
}
