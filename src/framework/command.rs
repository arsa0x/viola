use futures::future::BoxFuture;

use crate::framework::context::Context;
// use std::{future::Future, pin::Pin};

// pub type CommandFuture = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>>;

// pub type CommandHandler = fn(Context) -> CommandFuture;

pub type CommandHandler = fn(Context) -> BoxFuture<'static, anyhow::Result<()>>;

pub struct Command {
    pub triggers: &'static [&'static str],
    pub handler: CommandHandler,
}

inventory::collect!(Command);
