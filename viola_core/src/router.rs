use crate::{
    command::Command, context::Context, lua::lua_context::LuaContext, lua::lua_plugin::LuaPlugin,
};
use ahash::AHashMap;
use anyhow::anyhow;
use mlua::{Function, Lua};
use std::sync::Arc;

pub enum PluginKind {
    Native(&'static Command),
    Lua(Arc<LuaPlugin>),
}

pub struct Router {
    plugins: AHashMap<Arc<str>, PluginKind>,
    lua_vm: Lua,
}

impl Router {
    pub fn new() -> Self {
        let mut plugins = AHashMap::new();
        let lua_vm = Lua::new();

        if let Ok(lua_plugins) = LuaPlugin::load_plugins(&lua_vm) {
            for plugin in lua_plugins {
                let plugin = Arc::new(plugin);

                for trigger in &plugin.triggers {
                    if plugins.contains_key(trigger.as_ref()) {
                        log::warn!("duplicate trigger detected: {}", trigger);
                    }
                    plugins.insert(Arc::clone(trigger), PluginKind::Lua(Arc::clone(&plugin)));
                }
            }
        }

        for command in inventory::iter::<Command> {
            for trigger in command.triggers {
                let arced_trigger: Arc<str> = Arc::from(*trigger);

                if plugins.contains_key(&arced_trigger) {
                    log::warn!("duplicate trigger detected: {}", trigger);
                }
                plugins.insert(arced_trigger, PluginKind::Native(command));
            }
        }

        plugins.shrink_to_fit();

        Self { plugins, lua_vm }
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
                let lua_ctx = LuaContext { ctx: Arc::new(ctx) };

                let exec: Function = self.lua_vm.registry_value(&plugin.exec_key)?;

                exec.call_async::<()>(lua_ctx)
                    .await
                    .map_err(|e| anyhow!("plugin {}: {}", plugin.name, e))?;
            }
            PluginKind::Native(command) => {
                (command.handler)(ctx)
                    .await
                    .map_err(|e| anyhow!("plugin {}: {}", command.name, e))?;
            }
        }
        Ok(())
    }
}
