use viola_core::Context;
use viola_macros::command;
use whatsapp_rust::anyhow;

#[command(
    triggers = ["debug", "d"],
    owner_only = true,
    category = "tools",
)]
async fn debug(ctx: Context) -> anyhow::Result<()> {
    ctx.send()
        .interactive()
        .inapp_signup(&format!("message: \n\n```{:#?}```", ctx.message))
        .title("Message")
        .quoted()
        .await?;

    ctx.send()
        .interactive()
        .inapp_signup(&format!("info: \n\n```{:#?}```", ctx.info))
        .title("MessageInfo")
        .quoted()
        .await?;

    Ok(())
}
