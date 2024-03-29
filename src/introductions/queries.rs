use sqlx::PgPool;

#[derive(sqlx::FromRow)]
struct IntroUrl {
    yt_url: String,
}

pub async fn get_url_for_user_and_guild(
    user_id: u64,
    guild_id: u64,
    pool: PgPool,
) -> Option<String> {
    let user_id = user_id as i64;
    let guild_id = guild_id as i64;

    // shuttle doesn't support compile-time sqlx macros yet so we'll just use
    // them for testing. Must use string literal so can't abstract query.
    #[cfg(test)]
    let query = sqlx::query_as!(
        IntroUrl,
        r#"
        SELECT yt_url
        FROM intros
        WHERE user_snowflake = $1 AND guild_snowflake = $2
        "#,
        user_id,
        guild_id,
    );

    #[cfg(not(test))]
    let query = sqlx::query_as::<_, IntroUrl>(
        r#"
        SELECT yt_url
        FROM intros
        WHERE user_snowflake = $1 AND guild_snowflake = $2
        "#,
    )
    .bind(user_id)
    .bind(guild_id);

    query.fetch_one(&pool).await.ok().map(|url| url.yt_url)
}

pub async fn set_url_for_user_and_guild(
    user_id: u64,
    guild_id: u64,
    url: &str,
    pool: PgPool,
) -> Result<(), sqlx::Error> {
    let user_id = user_id as i64;
    let guild_id = guild_id as i64;

    #[cfg(test)]
    let query = sqlx::query!(
        r#"
        INSERT INTO intros (user_snowflake, guild_snowflake, yt_url)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_snowflake, guild_snowflake)
        DO UPDATE SET yt_url = EXCLUDED.yt_url
        "#,
        user_id,
        guild_id,
        url,
    );

    #[cfg(not(test))]
    let query = sqlx::query(
        r#"
        INSERT INTO intros (user_snowflake, guild_snowflake, yt_url)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_snowflake, guild_snowflake)
        DO UPDATE SET yt_url = EXCLUDED.yt_url
        "#,
    )
    .bind(user_id)
    .bind(guild_id)
    .bind(url);

    query.execute(&pool).await?;

    Ok(())
}

pub async fn clear_url_for_user_and_guild(
    user_id: u64,
    guild_id: u64,
    pool: PgPool,
) -> Result<(), sqlx::Error> {
    let user_id = user_id as i64;
    let guild_id = guild_id as i64;

    #[cfg(test)]
    let query = sqlx::query!(
        r#"
        DELETE FROM intros
        WHERE user_snowflake = $1 AND guild_snowflake = $2
        "#,
        user_id,
        guild_id,
    );

    #[cfg(not(test))]
    let query = sqlx::query(
        r#"
        DELETE FROM intros
        WHERE user_snowflake = $1 AND guild_snowflake = $2
        "#,
    )
    .bind(user_id)
    .bind(guild_id);

    query.execute(&pool).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database_setup::test_utils::get_test_database;

    const USER_SNOWFLAKE_1: u64 = 123456789123456789;
    const USER_SNOWFLAKE_2: u64 = 987654321987654321;
    const GUILD_SNOWFLAKE_1: u64 = 9876543219876543210;
    const GUILD_SNOWFLAKE_2: u64 = 1234567891234567890;
    const URL_1: &str = "https://example1.com";
    const URL_2: &str = "https://example2.com";

    #[tokio::test]
    async fn set_urls_are_retrieved() {
        // arrange
        let connection_pool = get_test_database().await;
        let test_cases = vec![
            (USER_SNOWFLAKE_1, GUILD_SNOWFLAKE_1, URL_1),
            (USER_SNOWFLAKE_2, GUILD_SNOWFLAKE_1, URL_2),
            (USER_SNOWFLAKE_1, GUILD_SNOWFLAKE_2, URL_2),
            (USER_SNOWFLAKE_2, GUILD_SNOWFLAKE_2, URL_1),
        ];

        for (user_snowflake, guild_snowflake, url) in test_cases {
            // act
            set_url_for_user_and_guild(
                user_snowflake,
                guild_snowflake,
                url,
                connection_pool.clone(),
            )
            .await
            .unwrap();
            let expected_url = get_url_for_user_and_guild(
                user_snowflake,
                guild_snowflake,
                connection_pool.clone(),
            )
            .await
            .unwrap();

            // assert
            assert_eq!(expected_url, url);
        }
    }

    #[tokio::test]
    async fn get_url_returns_none_when_no_url_set() {
        // arrange
        let connection_pool = get_test_database().await;

        // act
        let expected_url = get_url_for_user_and_guild(
            USER_SNOWFLAKE_1,
            GUILD_SNOWFLAKE_1,
            connection_pool.clone(),
        )
        .await;

        // assert
        assert!(expected_url.is_none());
    }

    #[tokio::test]
    async fn set_url_overwrites_existing_url() {
        let connection_pool = get_test_database().await;
        set_url_for_user_and_guild(
            USER_SNOWFLAKE_1,
            GUILD_SNOWFLAKE_1,
            URL_1,
            connection_pool.clone(),
        )
        .await
        .unwrap();
        set_url_for_user_and_guild(
            USER_SNOWFLAKE_1,
            GUILD_SNOWFLAKE_1,
            URL_2,
            connection_pool.clone(),
        )
        .await
        .unwrap();

        // act
        let expected_url = get_url_for_user_and_guild(
            USER_SNOWFLAKE_1,
            GUILD_SNOWFLAKE_1,
            connection_pool.clone(),
        )
        .await
        .unwrap();

        // assert
        assert_eq!(expected_url, URL_2);
    }

    #[tokio::test]
    async fn clear_url_removes_url() {
        // arrange
        let connection_pool = get_test_database().await;
        set_url_for_user_and_guild(
            USER_SNOWFLAKE_1,
            GUILD_SNOWFLAKE_1,
            URL_1,
            connection_pool.clone(),
        )
        .await
        .unwrap();

        // act
        clear_url_for_user_and_guild(USER_SNOWFLAKE_1, GUILD_SNOWFLAKE_1, connection_pool.clone())
            .await
            .unwrap();

        let expected_url = get_url_for_user_and_guild(
            USER_SNOWFLAKE_1,
            GUILD_SNOWFLAKE_1,
            connection_pool.clone(),
        )
        .await;

        // assert
        assert!(expected_url.is_none());
    }
}
