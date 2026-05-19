use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::anyhow;
use mlua::{Function, Lua, Table, UserData};

use crate::framework::context::Context;

#[derive(Clone)]
pub struct LuaContext {
    pub ctx: Arc<Context>,
}

impl UserData for LuaContext {
    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method("reply", |_, this, text: String| async move {
            this.ctx.reply(&text).await?;
            Ok(())
        });
        methods.add_method("elapsed_ms", |_, this, ()| Ok(this.ctx.elapsed_ms() as i64));
    }
}

pub struct LuaPlugin {
    pub triggers: Vec<String>,

    pub description: String,

    pub source: String,
}

impl LuaPlugin {
    fn collect_lua_files(dir: &Path, files: &mut Vec<PathBuf>) -> anyhow::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                LuaPlugin::collect_lua_files(&path, files)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("lua") {
                files.push(path);
            }
        }

        Ok(())
    }

    pub fn from_file(path: &Path) -> anyhow::Result<Self> {
        let source = fs::read_to_string(path)?;
        Self::from_source(source)
    }

    pub fn load_plugins() -> anyhow::Result<Vec<LuaPlugin>> {
        let plugins_path = dirs::data_dir()
            .ok_or_else(|| anyhow!("failed to get data dir"))?
            .join("viola")
            .join("plugins");

        if !plugins_path.exists() {
            fs::create_dir_all(&plugins_path)?;
        }

        let mut files = Vec::new();

        LuaPlugin::collect_lua_files(&plugins_path, &mut files)?;

        let mut plugins = Vec::new();

        for file in files {
            match LuaPlugin::from_file(&file) {
                Ok(plugin) => {
                    log::info!("loaded lua plugin: {}", file.display());
                    plugins.push(plugin);
                }
                Err(e) => {
                    log::error!("failed to load lua plugin {}: {}", file.display(), e);
                }
            }
        }

        Ok(plugins)
    }

    pub fn from_source(source: String) -> anyhow::Result<LuaPlugin> {
        let lua = Lua::new();

        let table: Table = lua.load(&source).eval()?;

        let triggers_table: mlua::Table = table.get("triggers")?;

        let mut triggers = Vec::new();

        for pair in triggers_table.sequence_values::<String>() {
            triggers.push(pair?);
        }

        let description: String = table.get("description").unwrap_or_default();

        let _: Function = table.get("exec")?;

        Ok(Self {
            triggers,
            description,
            source,
        })
    }

    pub async fn execute(&self, ctx: Context) -> anyhow::Result<()> {
        let lua = Lua::new();

        let lua_ctx = LuaContext { ctx: Arc::new(ctx) };

        let table: mlua::Table = lua.load(&self.source).eval()?;

        let exec: mlua::Function = table.get("exec")?;

        exec.call_async::<()>(lua_ctx).await?;

        Ok(())
    }
}

#[test]
fn lua_test() -> anyhow::Result<()> {
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

    let plugin = LuaPlugin::from_source(example)?;

    for cmd in plugin.triggers {
        println!("{cmd}");
    }

    Ok(())
}
