# Adventus

👋 [Add me to your server!](https://discord.com/oauth2/authorize?client_id=1074795024946036889)

## Features

Documentation for each slash command pops up in Discord when you type `/` in a message box.

### 🎲 Roller

A simple dice roller. Here are some examples:

- `/roll` (defaults to 1d20) returns: "`@user` rolled 10."
- `/roll sides: 20` returns: "`@user` rolled 13."
- `/roll sides: 6 rolls: 2` returns: "`@user` rolled 2, 5."

### 📯 Introductions

Plays a user's introduction sound when they join a voice channel. This is configurable with the following commands:

- `/set_intro` - Add an introduction sound from a YouTube URL
- `/clear_intro` - Remove your introduction sound

#### Notes

- Sounds are set on a per-guild basis.
- The YouTube video length can be up to 5 seconds long.
- The bot joins the voice channel (if not already present) to play the introduction sound.
- The bot will stick around so that subsequent user joins can be announced faster.
- The bot leaves the voice channel after 5 minutes of inactivity.

## How does it work?

The [previous incarnation of this bot](https://github.com/fng97/adventus/tree/0b9c31b675cc2f3c98eff944f6740f1e9b0f2cb8) used a serverless API and [Discord Interactions](https://discord.com/developers/docs/interactions/receiving-and-responding) to handle the `/roll` slash command without a Discord framework. I was pretty proud of that, but I wanted to add more features and learn more about Rust, so I decided to rewrite it.

What I'm using:

- 🦀 [Rust](https://www.rust-lang.org) ✨
- 🎙️ [Serenity](https://github.com/serenity-rs/serenity), [Poise](https://github.com/serenity-rs/poise), and [Songbird](https://github.com/serenity-rs/songbird) for the Discord client
- 🐘 [PostgreSQL](https://www.postgresql.org) and [`sqlx`](https://github.com/launchbadge/sqlx) for persistence
- 🚀 [Shuttle](https://www.shuttle.rs) for infrastructure
- 🐳 [Dev Container](https://containers.dev) for development
- 🪄 [GitHub Actions](https://github.com/features/actions) for CI/CD

Future improvements:

- cache audio for faster playback
- help command
- instrument with tracing spans
- replace hard-coded values with configuration

## Running Locally

You can get this running locally easily using Dev Containers. This assumes you have [Docker](https://www.docker.com) and [Visual Studio Code](https://code.visualstudio.com) installed, including the [Remote Containers](https://github.com/microsoft/vscode-remote-release) extension.

To set up the development environment:

1. Open Visual Studio Code
2. From the command palette, select "Dev Containers: Clone Repository in Container Volume..."
3. Enter `fng97/adventus`

Now just wait for the container to build before Visual Studio Code reloads with your development environment ready to go!

To run the tests:

1. Start the database: `./scripts/init_db.sh`
2. Run the tests: `cargo test`

To run the bot:

1. Ensure `Secrets.toml` includes your discord token in the format `DISCORD_TOKEN="your_token_here"`
2. Start the database if you haven't already: `./scripts/init_db.sh`
3. Run the bot locally: `cargo shuttle run`
