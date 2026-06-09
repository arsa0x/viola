use crate::lua::lua_http::LuaHttpClient;
use mlua::Lua;

pub fn setup_lua_environment(lua: Lua, client: reqwest::Client) -> anyhow::Result<Lua> {
    let lua_http = LuaHttpClient { client };

    lua.globals().set("http", lua_http)?;

    Ok(lua)
}
