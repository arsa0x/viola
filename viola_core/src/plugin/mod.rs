use std::sync::Arc;

use ahash::AHashMap;
use viola_script::{
    Chunk, Vm,
    error::{NativeError, NativeErrorKind},
    native::{ExecContext, Host, Value},
};
use whatsapp_rust::anyhow;

use crate::Context;

pub struct PluginHost {
    ctx: Context,
}

pub struct PluginRegistry {
    triggers: AHashMap<String, Arc<Chunk>>,
}

impl PluginRegistry {
    pub fn load(plugins_dir: &str) -> anyhow::Result<Self> {
        let mut triggers = AHashMap::new();

        for entry in std::fs::read_dir(plugins_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) != Some("vi") {
                continue;
            }

            let src = std::fs::read_to_string(&path)?;

            let chunk = viola_script::compile(&src)
                .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;

            if chunk.triggers.is_empty() {
                anyhow::bail!("{}: plugin does not have a trigger", path.display());
            }

            let chunk = Arc::new(chunk);

            for trigger in &chunk.triggers {
                if triggers
                    .insert(trigger.clone(), Arc::clone(&chunk))
                    .is_some()
                {
                    anyhow::bail!(
                        "trigger '{trigger}' is defined in more than one plugin ({})",
                        path.display()
                    );
                }

                log::info!("loaded plugin trigger '{trigger}' from {}", path.display());
            }
        }

        Ok(Self { triggers })
    }

    pub fn find(&self, trigger: &str) -> Option<Arc<Chunk>> {
        self.triggers.get(trigger).cloned()
    }

    pub fn is_empty(&self) -> bool {
        self.triggers.is_empty()
    }

    pub fn len(&self) -> usize {
        self.triggers.len()
    }
}

impl Host for PluginHost {
    async fn send_text(&self, text: &str) -> Result<(), NativeError> {
        _ = self.ctx.send().text(text).await.map_err(|e| NativeError {
            kind: NativeErrorKind::HostRejected,
            detail: e.to_string(),
        });

        Ok(())
    }
}

pub async fn dispatch(ctx: Context, chunk: &Chunk) -> anyhow::Result<()> {
    let args: Vec<Value> = ctx
        .args
        .iter()
        .skip(1)
        .map(|s| Value::Str(Arc::from(s.as_str())))
        .collect();

    let host = PluginHost { ctx };
    let exec_ctx = ExecContext::new(args, host);

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        Vm::new(chunk).run(&exec_ctx),
    )
    .await;

    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e.into()),
        Err(_) => Err(anyhow::anyhow!("script timeout")),
    }
}
