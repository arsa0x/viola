use crate::framework::{command::Command, context::Context, lua_plugin::LuaPlugin};
use std::{collections::HashMap, sync::Arc};

pub enum PluginKind {
    Native(&'static Command),
    Lua(Arc<LuaPlugin>),
}

pub struct Router {
    plugins: HashMap<String, PluginKind>,
}

impl Router {
    pub fn new() -> Self {
        let mut plugins = HashMap::new();

        if let Ok(lua_plugins) = LuaPlugin::load_plugins() {
            for plugin in lua_plugins {
                let plugin = Arc::new(plugin);

                for trigger in &plugin.triggers {
                    if plugins.contains_key(trigger) {
                        log::warn!("duplicate trigger detected: {}", trigger);
                    }

                    plugins.insert(trigger.clone(), PluginKind::Lua(Arc::clone(&plugin)));
                }
            }
        }

        for command in inventory::iter::<Command> {
            for trigger in command.triggers {
                if plugins.contains_key(*trigger) {
                    log::warn!("duplicate trigger detected: {}", trigger);
                }
                plugins.insert(trigger.to_string(), PluginKind::Native(command));
            }
        }

        Self { plugins }
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
                plugin.execute(ctx).await?;
            }
            PluginKind::Native(command) => {
                (command.handler)(ctx).await?;
            }
        }
        Ok(())
    }
}
