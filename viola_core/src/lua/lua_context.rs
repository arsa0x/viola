use crate::{
    context::{Context, MediaSource},
    utils::media_type_from_str,
};
use mlua::{Error as LuaError, UserData};
use std::sync::Arc;

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

        methods.add_async_method("reply_wait", |_, this, ()| async move {
            this.ctx.reply_wait().await.map_err(LuaError::external)?;
            Ok(())
        });

        methods.add_async_method("reply_success", |_, this, ()| async move {
            this.ctx.reply_success().await.map_err(LuaError::external)?;
            Ok(())
        });

        methods.add_async_method("reply_failed", |_, this, ()| async move {
            this.ctx.reply_failed().await.map_err(LuaError::external)?;
            Ok(())
        });

        methods.add_async_method(
            "reply_media_url",
            |_, this, (url, media_type_str, caption): (String, String, Option<String>)| async move {
                let media_type = media_type_from_str(&media_type_str).ok_or_else(|| {
                    LuaError::external(format!("invalid media type: {}", media_type_str))
                })?;

                let source = MediaSource::Url(url);

                this.ctx.reply_media(source, media_type, caption).await?;
                Ok(())
            },
        );

        methods.add_async_method(
            "reply_media_bytes",
            |_, this, (bytes, media_type_str, caption): (Vec<u8>, String, Option<String>)| async move {
                let media_type = media_type_from_str(&media_type_str).ok_or_else(|| {
                    LuaError::external(format!("invalid media type: {}", media_type_str))
                })?;

                let source = MediaSource::Bytes(bytes);

                this.ctx.reply_media(source, media_type, caption).await?;
                Ok(())
            },
        );

        methods.add_method("processing_ms", |_, this, ()| Ok(this.ctx.processing_ms()));

        methods.add_method("sender", |_, this, ()| {
            this.ctx.sender().map_err(LuaError::external)
        });

        methods.add_method("is_group", |_, this, ()| Ok(this.ctx.is_group()));

        methods.add_method("text_content", |_, this, ()| {
            Ok(this.ctx.text_content().map(|s| s.to_string()))
        });
    }
}
