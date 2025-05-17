pub mod commands {
    use crate::{Context, Error};

    use poise::serenity_prelude::Mentionable;
    use rand::Rng;

    /// Roll the dice!
    ///
    /// This command rolls the given number of dice with the given number of sides.
    /// The number of sides is 20 by default. The number of dice is optional and
    /// defaults to 1. The number of sides must be between 2 and 100, and the number
    /// of dice must be between 1 and 10.
    #[poise::command(slash_command)]
    pub async fn roll(
        ctx: Context<'_>,
        #[description = "Number of sides of the dice."]
        #[min = 2]
        #[max = 100]
        sides: Option<u8>,
        #[description = "Number of dice to roll."]
        #[min = 1]
        #[max = 10]
        rolls: Option<u8>,
    ) -> Result<(), Error> {
        const DEFAULT_NUM_ROLLS: u8 = 1;
        const DEFAULT_NUM_SIDES: u8 = 20;

        let rolls = rolls.unwrap_or(DEFAULT_NUM_ROLLS);
        let sides = sides.unwrap_or(DEFAULT_NUM_SIDES);

        let results: Vec<u8> = (0..rolls)
            .map(|_| rand::thread_rng().gen_range(1..=sides))
            .collect();

        let results_str = results
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(", ");

        ctx.say(format!(
            "🎲 {} rolled {}.",
            ctx.author().mention(),
            results_str
        ))
        .await?;

        Ok(())
    }
}
