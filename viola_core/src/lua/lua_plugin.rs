use anyhow::anyhow;
use mlua::{Function, Lua, RegistryKey, Table};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct LuaPlugin {
    pub name: Box<str>,
    pub triggers: Box<[Arc<str>]>,
    pub description: Box<str>,
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

    pub fn load_plugins(lua: &Lua) -> anyhow::Result<Vec<LuaPlugin>> {
        let plugins_path = dirs::data_dir()
            .ok_or_else(|| anyhow!("failed to get data dir"))?
            .join("viola")
            .join("plugins");

        if !plugins_path.exists() {
            fs::create_dir_all(&plugins_path)?;
        }

        let mut files = Vec::new();

        LuaPlugin::collect_lua_files(&plugins_path, &mut files)?;

        let mut plugins = Vec::with_capacity(files.len());

        for file in files {
            let source = fs::read_to_string(&file)?;
            match LuaPlugin::from_source(lua, source) {
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

    pub fn from_source(lua: &Lua, source: String) -> anyhow::Result<LuaPlugin> {
        let env = lua.create_table()?;

        let globals = lua.globals();

        let _ = env.set_metatable(Some(lua.create_table_from([("__index", globals)])?));

        let table: Table = lua.load(&source).set_environment(env.clone()).eval()?;

        let triggers_table: Table = table
            .get("triggers")
            .map_err(|_| anyhow!("plugin missing 'triggers' table"))?;

        let mut triggers_vec = Vec::with_capacity(triggers_table.raw_len());

        for value in triggers_table.sequence_values::<String>() {
            let trigger_string: String = value?;
            triggers_vec.push(Arc::from(trigger_string.into_boxed_str()));
        }

        let triggers = triggers_vec.into_boxed_slice();
        let name: Box<str> = table.get::<String>("name")?.into_boxed_str();
        let description: Box<str> = table
            .get::<String>("description")
            .unwrap_or_default()
            .into_boxed_str();

        let exec: Function = table.get("exec")?;
        let exec_key = lua.create_registry_value(exec)?;

        Ok(Self {
            name,
            triggers,
            description,
            exec_key,
        })
    }
}
