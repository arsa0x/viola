use anyhow::Context as AnyhowContext;
use isahc::{
    AsyncReadResponseExt,
    config::{Configurable, RedirectPolicy},
};
use url::{Url, form_urlencoded};
use viola_core::context::Context;
use viola_macros::command;
use whatsapp_rust::anyhow;

const UA: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:150.0) Gecko/20100101 Firefox/150.0";

const GET_HEADERS: [(&str, &str); 4] = [
    (
        "Accept",
        "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
    ),
    ("Accept-Language", "en-US,en;q=0.9"),
    ("Upgrade-Insecure-Requests", "1"),
    ("Sec-Fetch-Site", "same-origin"),
];

fn find_between<'a>(haystack: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let after_start = haystack.find(start)? + start.len();
    let rest = &haystack[after_start..];
    let end_idx = rest.find(end)?;
    Some(&rest[..end_idx])
}

fn unescape_html(input: &str) -> String {
    if !input.contains('&') {
        return input.to_string();
    }
    input
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#039;", "'")
}

fn extract_token(html: &str) -> Option<String> {
    find_between(
        html,
        r#"<input name="_token" type="hidden" value=""#,
        r#"">"#,
    )
    .map(unescape_html)
}

fn extract_first_href(html: &str) -> Option<String> {
    find_between(html, r#"<a href=""#, "\"").map(unescape_html)
}

fn join_set_cookies<I, S>(values: I) -> String
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    values
        .filter_map(|v| v.as_ref().split(';').next().map(str::to_string))
        .collect::<Vec<_>>()
        .join("; ")
}

pub async fn ouo_bypass(ctx: &Context, url: &str) -> anyhow::Result<Option<String>> {
    let url_obj = Url::parse(url)?;
    let domain = url_obj.host_str().context("failed getting domain")?;
    let origin = format!("{}://{}", url_obj.scheme(), domain);

    let mut init_req = ctx.http().raw("GET", url).header("User-Agent", UA);
    for (key, value) in GET_HEADERS {
        init_req = init_req.header(key, value);
    }

    let mut init_res = init_req.send().await?;
    if !init_res.status().is_success() {
        return Ok(None);
    }

    let cookies = join_set_cookies(
        init_res
            .headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|v| v.to_str().ok()),
    );

    let init_html = init_res.text().await?;
    let Some(token_value) = extract_token(&init_html) else {
        return Ok(None);
    };

    // --- Step 2: POST token untuk melewati gate ---
    let next_url = url.replacen(&format!("{domain}/"), &format!("{domain}/xreallcygo/"), 1);
    let referer = url.replacen(&format!("{domain}/"), &format!("{domain}/go/"), 1);

    let body = form_urlencoded::Serializer::new(String::new())
        .extend_pairs([("_token", token_value.as_str()), ("x-token", "")])
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

    let mut post_res = ctx.http_client.send_async(post_req.body(body)?).await?;

    if let Some(location) = post_res.headers().get("location") {
        if let Ok(location) = location.to_str() {
            return Ok(Some(location.to_string()));
        }
    }

    let final_html = post_res.text().await?;
    Ok(extract_first_href(&final_html))
}

const HELP: &str = "USAGE: .ouo <ouo_url>";

#[command(
    triggers = ["ouo"],
    help = HELP,
    category = "resolver",
    description = "ouo.io and ouo.press resolver"
)]
async fn ouo(ctx: Context) -> anyhow::Result<()> {
    let Some(url) = ctx.args.iter().find(|arg| {
        arg.starts_with("https://") && (arg.contains("ouo.io") || arg.contains("ouo.press"))
    }) else {
        ctx.send().text(HELP).await?;
        return Ok(());
    };

    ctx.send().wait().await?;

    match ouo_bypass(&ctx, url).await? {
        Some(result) => {
            ctx.send()
                .interactive()
                .inapp_signup(&result)
                .title("Success")
                .quoted()
                .await?;
            ctx.send().success().await?;
        }
        None => {
            ctx.send().failed().await?;
        }
    }

    Ok(())
}
