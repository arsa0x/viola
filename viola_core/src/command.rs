use linkme::distributed_slice;
use whatsapp_rust::anyhow;

use crate::Context;

pub type Execute =
    fn(
        ctx: Context,
    ) -> std::pin::Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'static>>;

// pub type Execute = fn(ctx: bot::Context) -> BoxFuture<'static, anyhow::Result<()>>;

pub struct Command {
    pub name: &'static str,
    pub triggers: &'static [&'static str],
    pub category: &'static str,
    pub help: Option<&'static str>,
    pub description: Option<&'static str>,
    pub group_only: bool,
    pub owner_only: bool,
    pub execute: Execute,
}

#[distributed_slice]
pub static COMMANDS: [Command];
