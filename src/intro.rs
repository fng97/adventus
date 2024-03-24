use crate::player::play;
use serenity::{
    all::{ChannelId, GuildId, UserId},
    client::Context,
};
use sqlx::PgPool;
use tracing::debug;

#[derive(sqlx::FromRow)]
struct IntroUrl {
    yt_url: String,
}

async fn get_url_for_user_and_guild(user_id: u64, guild_id: u64, pool: &PgPool) -> Option<String> {
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

    query.fetch_one(pool).await.ok().map(|url| url.yt_url)
}

pub async fn set_intro(guild_id: GuildId, user_id: UserId, url: String, pool: &PgPool) {
    let user_id = user_id.get() as i64;
    let guild_id = guild_id.get() as i64;

    sqlx::query!(
        r#"
        INSERT INTO intros (user_snowflake, guild_snowflake, yt_url)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_snowflake, guild_snowflake)
        DO UPDATE SET yt_url = $3
        "#,
        user_id,
        guild_id,
        url
    )
    .execute(pool)
    .await
    .unwrap();
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
        debug!("No intro found for user {} in guild {}", user_id, guild_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_utils::get_test_database;

    #[tokio::test]
    async fn gets_url_for_user_and_guild() {
        // arrange
        const USER_SNOWFLAKE: u64 = 123456789123456789;
        const GUILD_SNOWFLAKE: u64 = 987654321987654321;
        const URL: &str = "https://example.com";

        let connection_pool = get_test_database().await;

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

        // act

        let expected_url =
            get_url_for_user_and_guild(USER_SNOWFLAKE, GUILD_SNOWFLAKE, &connection_pool)
                .await
                .unwrap();

        // assert
        assert_eq!(expected_url, URL);
    }
}
