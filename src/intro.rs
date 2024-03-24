use serenity::{
    all::{ChannelId, GuildId, UserId, VoiceState},
    client::Context,
};
use sqlx::PgPool;
use tracing::debug;

pub fn user_joined_voice(
    ctx: &Context,
    old: &Option<VoiceState>,
    new: &VoiceState,
) -> Option<(ChannelId, GuildId, UserId)> {
    if new.user_id == ctx.cache.current_user().id {
        debug!("Bot joined the channel. Ignoring.");
        return None;
    }

    let guild_id = new.guild_id?;
    let channel_id = new.channel_id?;

    if old
        .as_ref()
        .and_then(|o| o.channel_id)
        .map(|old_channel_id| old_channel_id == channel_id)
        .unwrap_or(false)
    {
        debug!("State change within same channel. Ignoring.");
        return None;
    }

    Some((channel_id, guild_id, new.user_id))
}

#[derive(sqlx::FromRow)]
struct IntroUrl {
    yt_url: String,
}

pub async fn get_url_for_user_and_guild(
    user_id: u64,
    guild_id: u64,
    pool: &PgPool,
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

    query.fetch_one(pool).await.ok().map(|url| url.yt_url)
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
