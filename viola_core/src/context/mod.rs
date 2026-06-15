pub mod args;
pub mod info;
pub mod media;
pub mod message;
pub mod send;

use crate::state::AppState;
use compact_str::CompactString;
use std::{sync::Arc, time::Instant};
use whatsapp_rust::bot::MessageContext;

pub struct Context {
    pub msg_ctx: MessageContext,
    pub args: Vec<CompactString>,
    pub state: Arc<AppState>,
    pub created_at: Instant,
}
