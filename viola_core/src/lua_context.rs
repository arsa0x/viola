use mlua::UserData;
use std::sync::Arc;

use crate::context::Context;

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

        methods.add_method("elapsed_ms_f64", |_, this, ()| {
            Ok(this.ctx.elapsed_ms_f64() as f64)
        });
    }
}
