use viola_core::{Context, message::media::MediaSource};
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
                ctx.send()
                    .image(MediaSource::Bytes(download))
                    .quoted()
                    .await?;
                Ok(())
            }
            MediaType::Video => {
                ctx.send()
                    .video(MediaSource::Bytes(download))
                    .quoted()
                    .await?;
                Ok(())
            }
            MediaType::Audio => {
                ctx.send()
                    .audio(MediaSource::Bytes(download))
                    .quoted()
                    .await?;
                Ok(())
            }
            _ => ctx.send().failed().await,
        }
    } else {
        ctx.send().failed().await
    }
}
