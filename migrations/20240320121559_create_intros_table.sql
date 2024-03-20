-- create intros table
CREATE TABLE intros (
    user_snowflake BIGINT NOT NULL,
    guild_snowflake BIGINT NOT NULL,
    yt_url text NOT NULL,
    PRIMARY KEY (user_snowflake, guild_snowflake)
);