use crate::framework::context::Context;
use anyhow::Context as AnyhowContext;
use isahc::{Request, prelude::*};
use macros::command;
use scraper::{Html, Selector};
use url::Url;

pub async fn ouo_bypass(
    url: &str,
    redirect: isahc::config::RedirectPolicy,
) -> anyhow::Result<Option<String>> {
    let url_obj = Url::parse(url)?;
    let domain = url_obj.host_str().context("failed getting domain")?;
    let origin = format!("{}://{}", url_obj.scheme(), domain);
    let user_agent = "Mozilla/5.0 (X11; Linux x86_64; rv:150.0) Gecko/20100101 Firefox/150.0";

    let client = isahc::HttpClient::builder()
        .cookies()
        .redirect_policy(redirect)
        .build()?;

    let mut init_res = client
        .send_async(
            Request::get(url)
                .header("User-Agent", user_agent)
                .header(
                    "Accept",
                    "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
                )
                .header("Accept-Language", "en-US,en;q=0.9")
                .header("Upgrade-Insecure-Requests", "1")
                .header("Sec-Fetch-Site", "same-origin")
                .body(())?,
        )
        .await?;

    if !init_res.status().is_success() {
        return Ok(None);
    }

    let init_html = init_res.text().await?;

    let token = {
        let document = Html::parse_document(&init_html);

        let selector = Selector::parse(r#"input[name="_token"]"#).unwrap();

        document
            .select(&selector)
            .next()
            .and_then(|el| el.value().attr("value"))
            .map(str::to_string)
    };

    let token = match token {
        Some(v) => v,
        None => return Ok(None),
    };

    let next_url = url.replace(&format!("{}/", domain), &format!("{}/xreallcygo/", domain));
    let referer = url.replace(&format!("{}/", domain), &format!("{}/go/", domain));
    let body = format!("_token={}&x-token=", urlencoding::encode(&token));

    let mut post_res = client
        .send_async(
            Request::post(&next_url)
                .header("User-Agent", user_agent)
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
                .header("Priority", "u=0, i")
                .body(body)?,
        )
        .await?;

    let final_html = post_res.text().await?;
    let final_doc = Html::parse_document(&final_html);

    let a_selector = if redirect == isahc::config::RedirectPolicy::None {
        Selector::parse("a").unwrap()
    } else {
        Selector::parse("div.row a").unwrap()
    };

    let final_url = final_doc
        .select(&a_selector)
        .next()
        .and_then(|el| el.value().attr("href"))
        .map(str::to_string);

    Ok(final_url)
}

#[tokio::test]
async fn ouo_bypass_test() {
    let response = ouo_bypass(
        "https://ouo.io/0ZuHXU2",
        isahc::config::RedirectPolicy::None,
    )
    .await;
    assert!(response.unwrap().is_some());
}

#[command(trigger = ["ouo"])]
async fn ouo(ctx: Context) -> anyhow::Result<()> {
    let use_redirect = ctx.args.iter().any(|a| a == "-r" || a == "--redirect");
    let url = ctx.args.iter().find(|arg| {
        arg.starts_with("https://") && (arg.contains("ouo.io") || arg.contains("ouo.press"))
    });

    let Some(url) = url else {
        ctx.reply("usage: .ouo [-r] <ouo_url>").await?;
        return Ok(());
    };

    let redirect_policy = if use_redirect {
        isahc::config::RedirectPolicy::Follow
    } else {
        isahc::config::RedirectPolicy::None
    };

    match ouo_bypass(url, redirect_policy).await? {
        Some(result) => {
            ctx.reply(&format!("result: {}\ntime: {}ms", result, ctx.elapsed_ms()))
                .await?;
        }
        None => {
            ctx.reply("failed bypass").await?;
        }
    }

    Ok(())
}
