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
                commands.insert(*trigger, command);
            }
        }

        Self { commands }
    }

    pub async fn execute(&self, cmd: &str, ctx: Context) -> anyhow::Result<()> {
        let command = self.commands.get(cmd);
        if let Some(command) = command {
            (command.handler)(ctx).await?;
        }
        Ok(())
    }
}
