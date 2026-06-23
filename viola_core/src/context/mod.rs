pub mod args;
pub mod info;
pub mod media;

use crate::{
    context::{args::Args, info::Info, media::Media},
    message::MessageFactory,
    state::AppState,
};
use compact_str::CompactString;
use std::{sync::Arc, time::Instant};
use whatsapp_rust::bot::MessageContext;

pub struct Context {
    pub msg_ctx: MessageContext,
    pub args: Vec<CompactString>,
    pub state: Arc<AppState>,
    pub created_at: Instant,
}

impl Context {
    pub fn message(&self) -> MessageFactory<'_> {
        MessageFactory { ctx: self }
    }
    pub fn info(&self) -> Info<'_> {
        Info { ctx: self }
    }
    pub fn args(&self) -> Args<'_> {
        Args { ctx: self }
    }
    pub fn media(&self) -> Media<'_> {
        Media { ctx: self }
    }
}
