"""Adventus bot app."""

import discord
from discord.ext import commands
import random

import credentials

description = """Adventus bot ennit, dnd and all that jazz."""

intents = discord.Intents.default()
intents.members = True
intents.message_content = True

bot = commands.Bot(command_prefix="!", description=description, intents=intents)


@bot.event
async def on_ready():
    print(f"Logged in as {bot.user} (ID: {bot.user.id})")
    print("------")


@bot.command(pass_context=True)
async def roll(ctx, dice: str):
    """Rolls a dice in NdN format."""
    try:
        rolls, limit = map(int, dice.split("d"))
    except Exception:
        await ctx.send("Format has to be in NdN!")
        return

    result = ", ".join(str(random.randint(1, limit)) for r in range(rolls))
    msg = '{0.author.mention} rolled {1}.'.format(ctx.message, result)
    await ctx.send(msg)


bot.run(credentials.token)
