use crate::{
    command::{COMMANDS, Command},
    context::Context,
};
use ahash::AHashMap;
use anyhow::anyhow;
use unicase::UniCase;

pub struct Router {
    plugins: AHashMap<UniCase<&'static str>, &'static Command>,
}

impl Router {
    pub fn new() -> Self {
        let mut plugins = AHashMap::new();

        for command in COMMANDS {
            for trigger in command.triggers {
                let t = UniCase::new(*trigger);
                if plugins.contains_key(&t) {
                    log::warn!("duplicate trigger detected: {}", trigger);
                }
                plugins.insert(UniCase::new(*trigger), command);
            }
        }

        plugins.shrink_to_fit();

        Self { plugins }
    }

    fn _is_help_flag(&self, args: &[String]) -> bool {
        args.iter().any(|arg| arg == "--help")
    }

    pub async fn execute(&self, cmd: &str, ctx: Context) -> anyhow::Result<()> {
        let Some(plugin) = self.plugins.get(&UniCase::new(cmd)) else {
            return Ok(());
        };

        (plugin.handler)(ctx)
            .await
            .map_err(|e| anyhow!("plugin {}: {}", plugin.name, e))?;

        Ok(())
    }
}
