use std::sync::Arc;

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
    }
}

pub struct LuaPlugin {
    pub triggers: Vec<String>,

    pub description: String,

    pub source: String,
}

impl LuaPlugin {
    pub fn from_source(source: String) -> anyhow::Result<Self> {
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
