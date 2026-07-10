use crate::{
    command::{COMMANDS, Command},
    config::BotMode,
    context::Context,
};
use ahash::AHashMap;
use anyhow::anyhow;
use unicase::UniCase;

pub struct Router {
    plugins: AHashMap<UniCase<&'static str>, &'static Command>,
}

impl Router {
    pub fn new() -> Self {
        let mut plugins = AHashMap::new();

        for command in COMMANDS {
            for trigger in command.triggers {
                let t = UniCase::new(*trigger);
                if plugins.contains_key(&t) {
                    log::warn!("duplicate trigger detected: {}", trigger);
                }
                plugins.insert(UniCase::new(*trigger), command);
            }
        }

        plugins.shrink_to_fit();

        Self { plugins }
    }

    fn is_help_flag(&self, args: &[String]) -> bool {
        args.iter().any(|arg| arg == "--help")
    }

    pub async fn execute(&self, cmd: &str, ctx: Context) -> anyhow::Result<()> {
        let Some(plugin) = self.plugins.get(&UniCase::new(cmd)) else {
            return Ok(());
        };

        let config = ctx.state.read_config().await;
        let is_owner = ctx.info().is_owner(&config);

        match config.bot.mode {
            BotMode::Disabled => return Ok(()),
            BotMode::Group => {
                if !ctx.info().is_group() {
                    return Ok(());
                }
            }
            BotMode::Owner => {
                if !is_owner {
                    return Ok(());
                }
            }
            BotMode::Public => {}
        }

        if self.is_help_flag(ctx.args.as_slice()) {
            let triggers = plugin
                .triggers
                .iter()
                .map(|f| format!(".{}", f))
                .collect::<Vec<_>>()
                .join(", ");
            let description = if plugin.description.is_empty() {
                "no description"
            } else {
                plugin.description
            };
            let help = if plugin.help.is_empty() {
                "no help available"
            } else {
                plugin.help
            };

            ctx.send()
                .text(format!(
                    "{}\n\nALIASES: {}\n\n{}",
                    description, triggers, help
                ))
                .quoted()
                .await?;
            return Ok(());
        }

        if plugin.group_only && !ctx.info().is_group() {
            ctx.send()
                .text("command only works in groups")
                .quoted()
                .await?;
            return Ok(());
        }

        if plugin.owner && !is_owner {
            ctx.send().text("owner only command").quoted().await?;
            return Ok(());
        }

        (plugin.handler)(ctx)
            .await
            .map_err(|e| anyhow!("plugin {}: {}", plugin.name, e))?;

        Ok(())
    }
}
