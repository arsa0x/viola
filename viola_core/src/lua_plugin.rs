use crate::{context::Context, lua_context::LuaContext};
use anyhow::anyhow;
use mlua::{Function, Lua, RegistryKey, Table};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct LuaPlugin {
    pub triggers: Vec<String>,
    pub description: String,

    pub lua: Arc<Lua>,
    pub exec_key: RegistryKey,
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
        let lua = Arc::new(Lua::new());

        let table: Table = lua.load(&source).eval()?;
        let triggers_table: mlua::Table = table.get("triggers")?;

        let mut triggers = Vec::new();
        for value in triggers_table.sequence_values::<String>() {
            triggers.push(value?);
        }
        let description: String = table.get("description").unwrap_or_default();

        let exec: Function = table.get("exec")?;
        let exec_key = lua.create_registry_value(exec)?;

        Ok(Self {
            triggers,
            description,
            lua,
            exec_key,
        })
    }

    pub async fn execute(&self, ctx: Context) -> anyhow::Result<()> {
        let lua_ctx = LuaContext { ctx: Arc::new(ctx) };
        let exec: Function = self.lua.registry_value(&self.exec_key)?;
        exec.call_async::<()>(lua_ctx).await?;

        Ok(())
    }
}
