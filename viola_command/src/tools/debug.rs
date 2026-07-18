use viola_core::Context;
use viola_macros::command;
use whatsapp_rust::anyhow::{self, anyhow};

#[command(
    triggers = ["debug", "d"],
    owner_only = true,
    category = "tools",
)]
async fn debug(ctx: Context) -> anyhow::Result<()> {
    ctx.send()
        .text(&format!("message: \n\n```{:#?}```", ctx.message))
        .await?;
    ctx.send()
        .text(&format!("info: \n\n```{:#?}```", ctx.info))
        .await?;
    ctx.send()
        .text(&format!(
            "lid: \n\n```{:#?}```",
            ctx.wa_client.get_lid().ok_or_else(|| anyhow!("error"))?
        ))
        .await?;
    Ok(())
}
