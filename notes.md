# Notes

## Todos

- [x] get it connecting and playing
- [x] get it working as a shuttle project
- [ ] check that the track is less than a limit before playing
- [ ] don't let it go to sleep (remove shuttle timeout)
- [ ] command for adding a track for a user
  - [ ] validate URL
  - [ ] add postgres (sqlx) database to shuttle project
- [ ] check for unused dependencies
- [ ] move logic into separate modules
- [ ] cache audio
- [ ] set up telemetry with Grafana Cloud

- environemtn setup
  - had to install `shfmt` with `apt`
    - use [feature](https://github.com/devcontainers-contrib/features/tree/main/src/shfmt)?
  - `cargo install sqlx-cli --no-default-features --features postgres`
  - `apt install postgresql-client`
- add CI checks
  - shfmt
  - shellcheck

## To Add To README

- Using `build.rs` as a workaround for not being able to add dependencies to shuttle runtime. Need `yt-dlp` to play tracks.

## ChatGPT SQL Suggestions

Starting with PostgreSQL for your Discord bot, a more efficient approach than creating a table per server would be to use a relational model with just a few tables to handle all servers and users. This model will be more scalable and easier to maintain. Instead of creating a separate table for each server, you can create a structure that accommodates multiple servers in a single, well-designed schema.

Here's a simple design to get you started:

### 1. Servers Table

This table stores information about each server (often called "guilds" in Discord terminology).

- **ServerID** (Primary Key): Unique identifier for the server.
- **ServerName**: Name of the server.

```sql
CREATE TABLE servers (
    server_id BIGINT PRIMARY KEY,
    server_name VARCHAR(255) NOT NULL
);
```

### 2. Users Table

This table stores information about the users.

- **UserID** (Primary Key): Unique identifier for the user.
- **UserName**: The user's name or any other relevant information.

```sql
CREATE TABLE users (
    user_id BIGINT PRIMARY KEY,
    user_name VARCHAR(255) NOT NULL
);
```

### 3. UserServerURLs Table

This table links users with servers and their respective URLs. It handles the scenario where a user can have different URLs in different servers.

- **UserID**: References the `UserID` from the Users table.
- **ServerID**: References the `ServerID` from the Servers table.
- **URL**: The URL associated with the user for that server.

This table has a composite primary key consisting of `UserID` and `ServerID`, assuming a user can only have one URL per server.

```sql
CREATE TABLE user_server_urls (
    user_id BIGINT,
    server_id BIGINT,
    url VARCHAR(255) NOT NULL,
    PRIMARY KEY (user_id, server_id),
    FOREIGN KEY (user_id) REFERENCES users(user_id),
    FOREIGN KEY (server_id) REFERENCES servers(server_id)
);
```

### Advantages of This Approach

- **Scalability**: It's easier to manage and scale your database without needing to create new tables for every server. Adding a new server or user is just a matter of inserting a new row into the existing tables.
- **Maintenance**: With a more normalized design, maintaining your database and applying updates becomes simpler. You won't have to make the same change in multiple tables.
- **Performance**: Queries to retrieve a user's URL for a server, or to find all URLs associated with a user across servers, can be executed efficiently using joins.

### Conclusion

This schema provides a solid foundation that you can extend as your needs grow. For example, if you later decide to store additional information about each URL, you could easily add more columns to the `user_server_urls` table or create new tables and link them with foreign keys. Starting with a relational approach in PostgreSQL allows your database to grow and evolve with your application.
