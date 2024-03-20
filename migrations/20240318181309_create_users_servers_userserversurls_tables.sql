-- create users table
CREATE TABLE users (
    id uuid NOT NULL,
    PRIMARY KEY (id),
    snowflake BIGINT NOT NULL UNIQUE,
    registered_at timestamptz NOT NULL
);
-- create guilds table
CREATE TABLE guilds (
    id uuid NOT NULL,
    PRIMARY KEY (id),
    snowflake BIGINT NOT NULL UNIQUE,
    registered_at timestamptz NOT NULL
);
-- create user_guild_urls table
CREATE TABLE users_guilds_urls (
    user_id BIGINT NOT NULL,
    guild_id BIGINT NOT NULL,
    url text NOT NULL,
    PRIMARY KEY (user_id, guild_id),
);