use crate::context::Context;
use futures::future::BoxFuture;
use linkme::distributed_slice;
use std::time::Duration;

pub type CommandHandler = fn(Context) -> BoxFuture<'static, anyhow::Result<()>>;

pub struct Command {
    pub name: &'static str,
    pub triggers: &'static [&'static str],
    pub description: &'static str,
    pub category: &'static str,
    pub help: &'static str,
    pub cooldown: Duration,
    pub owner: bool,
    pub group_only: bool,
    pub handler: CommandHandler,
}

#[distributed_slice]
pub static COMMANDS: [Command];
