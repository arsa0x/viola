use anyhow::Context as AnyhowContext;
use isahc::{
    AsyncReadResponseExt,
    config::{Configurable, RedirectPolicy},
};
use url::{Url, form_urlencoded};
use viola_core::context::Context;
use viola_macros::command;

const UA: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:150.0) Gecko/20100101 Firefox/150.0";
const HEADERS: [(&str, &str); 5] = [
    ("User-Agent", UA),
    (
        "Accept",
        "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
    ),
    ("Accept-Language", "en-US,en;q=0.9"),
    ("Upgrade-Insecure-Requests", "1"),
    ("Sec-Fetch-Site", "same-origin"),
];

pub async fn ouo_bypass(ctx: &Context, url: &str) -> anyhow::Result<Option<String>> {
    let url_obj = Url::parse(url)?;
    let domain = url_obj.host_str().context("failed getting domain")?;

    let origin = format!("{}://{}", url_obj.scheme(), domain);

    let mut init_req = ctx.state.request("GET", url);

    for (key, value) in HEADERS {
        init_req = init_req.header(key, value);
    }

    let mut init_res = ctx.state.send(init_req).await?;

    let cookies = init_res
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .filter_map(|cookie| cookie.split(';').next())
        .collect::<Vec<_>>()
        .join("; ");

    if !init_res.status().is_success() {
        return Ok(None);
    }

    let init_html = init_res.text().await?;

    let token_value = {
        let document = tl::parse(&init_html, tl::ParserOptions::default())?;
        let parser = document.parser();

        let token_node = document
            .query_selector(r#"input[name="_token"]"#)
            .context("failed to execute selector query")?
            .next()
            .and_then(|handle| handle.get(parser))
            .and_then(|node| node.as_tag());

        match token_node {
            Some(tag) => match tag.attributes().get("value") {
                Some(Some(val)) => val.as_utf8_str().into_owned(),
                _ => return Ok(None),
            },
            None => return Ok(None),
        }
    };

    let next_url = url.replace(&format!("{}/", domain), &format!("{}/xreallcygo/", domain));
    let referer = url.replace(&format!("{}/", domain), &format!("{}/go/", domain));
    let form_data = [("_token", token_value.as_str()), ("x-token", "")];
    let body_string = form_urlencoded::Serializer::new(String::new())
        .extend_pairs(form_data)
        .finish();

    let mut post_req = isahc::Request::builder()
        .redirect_policy(RedirectPolicy::None)
        .method("POST")
        .uri(&next_url)
        .header("User-Agent", UA)
        .header(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .header("Accept-Language", "en-US,en;q=0.9")
        .header("Referer", referer)
        .header("Origin", origin)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Upgrade-Insecure-Requests", "1")
        .header("Sec-Fetch-Site", "same-origin")
        .header("Sec-Fetch-Dest", "document")
        .header("Sec-Fetch-Mode", "navigate")
        .header("Sec-Fetch-User", "?1")
        .header("Priority", "u=0, i");

    if !cookies.is_empty() {
        post_req = post_req.header("Cookie", cookies);
    }

    let mut post_res = ctx
        .state
        .http
        .send_async(post_req.body(body_string)?)
        .await?;

    let final_html = post_res.text().await?;

    let final_url = {
        let final_doc = tl::parse(&final_html, tl::ParserOptions::default())?;
        let final_parser = final_doc.parser();

        final_doc
            .query_selector("a")
            .context("failed final link selector query")?
            .next()
            .and_then(|handle| handle.get(final_parser))
            .and_then(|node| node.as_tag())
            .and_then(|tag| tag.attributes().get("href"))
            .flatten()
            .map(|val| val.as_utf8_str().into_owned())
    };
    Ok(final_url)
}

const HELP: &str = "USAGE: .ouo <ouo_url>";

#[command(trigger = ["ouo"], help = HELP)]
async fn ouo(ctx: Context) -> anyhow::Result<()> {
    let url = ctx.args.iter().find(|arg| {
        arg.starts_with("https://") && (arg.contains("ouo.io") || arg.contains("ouo.press"))
    });

    let Some(url) = url else {
        ctx.send().reply_text(HELP).await?;
        return Ok(());
    };

    ctx.send().reply_wait().await?;

    match ouo_bypass(&ctx, url).await? {
        Some(result) => {
            ctx.send().reply_text(&result).await?;
            ctx.send().reply_success().await?;
        }
        None => {
            ctx.send().reply_failed().await?;
        }
    }

    Ok(())
}
