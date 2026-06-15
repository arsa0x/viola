use crate::{
    command::{COMMANDS, Command},
    context::Context,
};
use ahash::AHashMap;
use anyhow::anyhow;
use compact_str::CompactString;
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

    fn is_help_flag(&self, args: &[CompactString]) -> bool {
        args.iter().any(|arg| arg == "--help")
    }

    pub async fn execute(&self, cmd: &str, ctx: Context) -> anyhow::Result<()> {
        let Some(plugin) = self.plugins.get(&UniCase::new(cmd)) else {
            return Ok(());
        };

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

            ctx.reply_text(&format!(
                "{}\n\nALIASES: {}\n\n{}",
                description, triggers, help
            ))
            .await?;
            return Ok(());
        }

        if plugin.group_only && !ctx.is_group() {
            ctx.reply_text("command only works in groups").await?;
            return Ok(());
        }

        if plugin.owner && ctx.sender_str()? != ctx.state.config.bot.owner {
            ctx.reply_text("owner only command").await?;
            return Ok(());
        }

        (plugin.handler)(ctx)
            .await
            .map_err(|e| anyhow!("plugin {}: {}", plugin.name, e))?;

        Ok(())
    }
}
