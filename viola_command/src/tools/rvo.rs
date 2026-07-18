use viola_core::Context;
use viola_macros::command;
use whatsapp_rust::{anyhow, download::MediaType};

#[command(
    triggers = ["rvo", "read", "show", "view"],
    category = "tools",
    description = "Read view once message"
)]
async fn read_view_once(ctx: Context) -> anyhow::Result<()> {
    if let Ok((mtype, media)) = ctx.get_quoted_media() {
        let download = ctx.wa_client.download(media).await?;
        match mtype {
            MediaType::Image => {
                ctx.send().image(download).quoted().await?;
                Ok(())
            }
            MediaType::Video => {
                ctx.send().video(download).quoted().await?;
                Ok(())
            }
            MediaType::Audio => {
                ctx.send().audio(download).quoted().await?;
                Ok(())
            }
            _ => ctx.send().failed().await,
        }
    } else {
        ctx.send().failed().await
    }
}
