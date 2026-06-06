use std::time::Duration;

use crate::context::Context;

use futures::future::BoxFuture;

pub type CommandHandler = fn(Context) -> BoxFuture<'static, anyhow::Result<()>>;

pub struct Command {
    pub name: &'static str,
    pub triggers: &'static [&'static str],
    pub description: &'static str,
    pub help: &'static str,
    pub cooldown: Duration,
    pub owner: bool,
    pub group_only: bool,
    pub handler: CommandHandler,
}

inventory::collect!(Command);
