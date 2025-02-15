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

NOTE: _This functionality is currently disabled. YouTube is not happy with my usage
of yt-dlp..._ 👀

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

The [previous incarnation of this bot](https://github.com/fng97/adventus/tree/86a6bd3099d72eebba1c7738ce1e167f975be48a) was written in Rust. It used the excellent [Serenity](https://github.com/serenity-rs/serenity), [Poise](https://github.com/serenity-rs/poise), and [Songbird](https://github.com/serenity-rs/songbird) frameworks for the Discord client. This version introduced _Introductions_. This was me trying out Rust.

The [original version of this bot](https://github.com/fng97/adventus/tree/0b9c31b675cc2f3c98eff944f6740f1e9b0f2cb8) used a serverless HTTP API to respond to [Discord Interactions](https://discord.com/developers/docs/interactions/receiving-and-responding) (e.g. `/roll`) without a Discord framework. Check out the architecture docs in the README for that commit. I was pretty proud of that one because it was simple and would never make it out of free tier.

What I'm using:

(Or rather what I _will_ use. The Rust bot is what's live now. This is a work in progress.)

- ⚡ [Zig](https://ziglang.org/)
- ❄️ [Nix](https://nixos.org/) for reproducible, declaritive builds and development environments
- 🪶 [SQLite](https://www.sqlite.org) for persistence
- 🪄 [GitHub Actions](https://github.com/features/actions) for CI/CD
