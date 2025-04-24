# Adventus

👋 [Add me to your
server!](https://discord.com/oauth2/authorize?client_id=1074795024946036889)

## Features

Documentation for each slash command pops up in Discord when you type `/` in a
message box.

### 🎲 Roller

A simple dice roller. Here are some examples:

- `/roll` (defaults to 1d20) returns: "`@user` rolled 10."
- `/roll sides: 20` returns: "`@user` rolled 13."
- `/roll sides: 6 rolls: 2` returns: "`@user` rolled 2, 5."

### 📯 Introductions

Plays a user's introduction sound when they join a voice channel. This is
configurable with the following commands:

- `/set_intro` - Add an introduction sound from an attachment
- `/clear_intro` - Remove your introduction sound

#### Notes

- Sounds are set on a per-guild basis.
- The attachment video/audio length can be up to 5 seconds long.
- The bot joins the voice channel (if not already present) to play the
  introduction sound.
- The bot will stick around so that subsequent user joins can be announced
  faster.
- The bot leaves the voice channel after 5 minutes of inactivity.

## How does it work?

The [previous incarnation of this
bot](https://github.com/fng97/adventus/tree/0b9c31b675cc2f3c98eff944f6740f1e9b0f2cb8)
used a serverless API and [Discord
Interactions](https://discord.com/developers/docs/interactions/receiving-and-responding)
to handle the `/roll` slash command without a Discord framework. I was pretty
proud of that, but I wanted to add more features and learn more about Rust so
(as they say) I rewrote it in Rust.

This version uses [Serenity](https://github.com/serenity-rs/serenity),
[Poise](https://github.com/serenity-rs/poise), and
[Songbird](https://github.com/serenity-rs/songbird) for the Discord client.
They, along with `ffmpeg` and `libopus`, do all the heavy lifting.

## Running Locally

Assuming you have Nix installed you can run the bot with:

```bash
nix develop --command bash -c 'INTROS_DIR="intros" DISCORD_TOKEN="<token>" cargo run'
```
