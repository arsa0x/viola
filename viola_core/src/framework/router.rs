use crate::framework::{command::Command, context::Context, lua_context::LuaPlugin};
use std::{collections::HashMap, sync::Arc};

pub enum PluginKind {
    Native(&'static Command),
    Lua(Arc<LuaPlugin>),
}

pub struct Router {
    // commands: HashMap<&'static str, &'static Command>,
    // cooldowns: DashMap<(String, &'static str), Instant>,
    plugins: HashMap<String, PluginKind>,
}

impl Router {
    pub fn new() -> Self {
        let mut plugins = HashMap::new();

        let example = String::from(
            r#"return {
            triggers = { "lua", "pl" },
            description = "ping command",
            exec = function(ctx)
                ctx:reply(
                    "pong from lua!"
                )
            end
        }"#,
        );

        let lua_source: LuaPlugin = LuaPlugin::from_source(example).unwrap();

        plugins.insert("lua".to_string(), PluginKind::Lua(Arc::new(lua_source)));

        for command in inventory::iter::<Command> {
            for trigger in command.triggers {
                if plugins.contains_key(*trigger) {
                    log::warn!("duplicate trigger detected: {}", trigger);
                }
                plugins.insert(trigger.to_string(), PluginKind::Native(command));
            }
        }

        Self {
            plugins,
            // cooldowns: DashMap::new(),
        }
    }

    fn _is_help_flag(&self, args: &[String]) -> bool {
        args.iter().any(|arg| arg == "--help")
    }

    pub async fn execute(&self, cmd: &str, ctx: Context) -> anyhow::Result<()> {
        let Some(plugin) = self.plugins.get(cmd) else {
            return Ok(());
        };

        match plugin {
            PluginKind::Lua(plugin) => {
                plugin.execute(ctx.clone()).await?;
            }
            PluginKind::Native(command) => {
                (command.handler)(ctx.clone()).await?;
            }
        }

        // let command = self.commands.get(cmd);
        // if let Some(command) = command {
        //     if self.is_help_flag(&ctx.args) {
        //         let triggers = command
        //             .triggers
        //             .iter()
        //             .map(|f| format!(".{}", f))
        //             .collect::<Vec<_>>()
        //             .join(", ");
        //         let description = if command.description.is_empty() {
        //             "no description"
        //         } else {
        //             command.description
        //         };
        //         let help = if command.help.is_empty() {
        //             "no help available"
        //         } else {
        //             command.help
        //         };

        //         ctx.reply(&format!(
        //             "{}\n\nALIASES: {}\n\n{}",
        //             description, triggers, help
        //         ))
        //         .await?;
        //         return Ok(());
        //     }

        //     let semaphore = Arc::clone(&ctx.state.semaphore);
        //     let _permit = semaphore.acquire().await?;

        //     // let sender = ctx.sender()?;
        //     // let cache_key = (sender, cmd);

        //     // if let Some(last_execution) = self.cooldowns.get(&cache_key) {
        //     //     if last_execution.elapsed() < command.cooldown {
        //     //         let remaining = command.cooldown - last_execution.elapsed();
        //     //         ctx.reply(&format!(
        //     //             "wait {:.1} more seconds!",
        //     //             remaining.as_secs_f32()
        //     //         ))
        //     //         .await?;
        //     //         return Ok(());
        //     //     }
        //     // }

        //     if command.group_only && !ctx.is_group() {
        //         ctx.reply("command only works in groups").await?;
        //         return Ok(());
        //     }
        //     if command.owner && ctx.sender()? != ctx.state.config.bot.owner {
        //         ctx.reply("owner only command").await?;
        //         return Ok(());
        //     }

        //     if let Err(e) = (command.handler)(ctx.clone()).await {
        //         log::error!("command failed: {}", e);

        //         let _ = ctx.reply(&format!("command failed: {}", e)).await;
        //         return Err(e);
        //     }

        //     // self.cooldowns.insert(cache_key, Instant::now());
        // }
        Ok(())
    }
}
