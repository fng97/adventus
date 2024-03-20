-- create intros table
CREATE TABLE intros (
    user_snowflake BIGINT NOT NULL,
    guild_snowflake BIGINT NOT NULL,
    yt_url text NOT NULL,
    set_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_snowflake, guild_snowflake)
);