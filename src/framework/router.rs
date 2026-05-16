use crate::framework::{command::Command, context::Context};
use std::collections::HashMap;

pub struct Router {
    commands: HashMap<&'static str, &'static Command>,
}

impl Router {
    pub fn new() -> Self {
        let mut commands = HashMap::new();

        for command in inventory::iter::<Command> {
            for trigger in command.triggers {
                if commands.contains_key(trigger) {
                    log::warn!("duplicate trigger detected: {}", trigger);
                }

                commands.insert(*trigger, command);
            }
        }

        Self { commands }
    }

    pub async fn execute(&self, cmd: &str, ctx: Context) -> anyhow::Result<()> {
        let command = self.commands.get(cmd);
        if let Some(command) = command {
            if command.group_only && !ctx.is_group() {
                ctx.reply("command only works in groups").await?;
                return Ok(());
            }
            if command.owner && ctx.sender()? != ctx.state.config.bot.owner {
                ctx.reply("owner only command").await?;
                ctx.reply(&format!("owner: {}", ctx.state.config.bot.owner))
                    .await?;
                return Ok(());
            }
            (command.handler)(ctx).await?;
        }
        Ok(())
    }
}
