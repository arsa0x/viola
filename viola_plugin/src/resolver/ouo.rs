// use anyhow::Context as AnyhowContext;
// use reqwest::Url;
// use viola_core::context::Context;
// use viola_macros::command;

// pub async fn ouo_bypass(
//     client: &reqwest::Client,
//     url: &str,
//     follow_redirect: bool,
// ) -> anyhow::Result<Option<String>> {
//     let url_obj = Url::parse(url)?;
//     let domain = url_obj.host_str().context("failed getting domain")?;
//     println!("start resolver: {}", url);
//     let origin = format!("{}://{}", url_obj.scheme(), domain);
//     let user_agent = "Mozilla/5.0 (X11; Linux x86_64; rv:150.0) Gecko/20100101 Firefox/150.0";

//     let init_res = client
//         .get(url)
//         .header("User-Agent", user_agent)
//         .header(
//             "Accept",
//             "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
//         )
//         .header("Accept-Language", "en-US,en;q=0.9")
//         .header("Upgrade-Insecure-Requests", "1")
//         .header("Sec-Fetch-Site", "same-origin")
//         .send()
//         .await?;

//     println!("status: {}", init_res.status());

//     if !init_res.status().is_success() {
//         return Ok(None);
//     }

//     let init_html = init_res.text().await?;

//     println!("html: {}", init_html);

//     let token_value = {
//         let document = tl::parse(&init_html, tl::ParserOptions::default())?;
//         let parser = document.parser();

//         let token_node = document
//             .query_selector(r#"input[name="_token"]"#)
//             .context("failed to execute selector query")?
//             .next()
//             .and_then(|handle| handle.get(parser))
//             .and_then(|node| node.as_tag());

//         println!("token: {:?}", token_node);

//         match token_node {
//             Some(tag) => match tag.attributes().get("value") {
//                 Some(Some(val)) => val.as_utf8_str().into_owned(),
//                 _ => return Ok(None),
//             },
//             None => return Ok(None),
//         }
//     };

//     let next_url = url.replace(&format!("{}/", domain), &format!("{}/xreallcygo/", domain));
//     let referer = url.replace(&format!("{}/", domain), &format!("{}/go/", domain));
//     let form_data = [("_token", token_value.as_str()), ("x-token", "")];

//     let post_res = client
//         .post(&next_url)
//         .header("User-Agent", user_agent)
//         .header(
//             "Accept",
//             "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
//         )
//         .header("Accept-Language", "en-US,en;q=0.9")
//         .header("Referer", referer)
//         .header("Origin", origin)
//         .header("Content-Type", "application/x-www-form-urlencoded")
//         .header("Upgrade-Insecure-Requests", "1")
//         .header("Sec-Fetch-Site", "same-origin")
//         .header("Sec-Fetch-Dest", "document")
//         .header("Sec-Fetch-Mode", "navigate")
//         .header("Sec-Fetch-User", "?1")
//         .header("Priority", "u=0, i")
//         .form(&form_data)
//         .send()
//         .await?;

//     if !post_res.status().is_success() {
//         return Ok(None);
//     }

//     let final_html = post_res.text().await?;

//     let final_url = {
//         let final_doc = tl::parse(&final_html, tl::ParserOptions::default())?;
//         let final_parser = final_doc.parser();
//         let selector_str = if follow_redirect { "a" } else { "div.row a" };

//         final_doc
//             .query_selector(selector_str)
//             .context("failed final link selector query")?
//             .next()
//             .and_then(|handle| handle.get(final_parser))
//             .and_then(|node| node.as_tag())
//             .and_then(|tag| tag.attributes().get("href"))
//             .flatten()
//             .map(|val| val.as_utf8_str().into_owned())
//     };

//     Ok(final_url)
// }

// const HELP: &str = "USAGE: .ouo [-r|--redirect] <ouo_url>";

// #[command(trigger = ["ouo"], help = HELP)]
// async fn ouo(ctx: Context) -> anyhow::Result<()> {
//     let follow_redirect = ctx.args.iter().any(|a| a == "-r" || a == "--redirect");

//     let mut client_builder = reqwest::Client::builder().cookie_store(true);

//     if !follow_redirect {
//         client_builder = client_builder.redirect(reqwest::redirect::Policy::none());
//     }
//     let client = client_builder.build()?;

//     let url = ctx.args.iter().find(|arg| {
//         arg.starts_with("https://") && (arg.contains("ouo.io") || arg.contains("ouo.press"))
//     });

//     let Some(url) = url else {
//         ctx.reply(HELP).await?;
//         return Ok(());
//     };

//     match ouo_bypass(&client, url, follow_redirect).await? {
//         Some(result) => {
//             ctx.reply(&result).await?;
//         }
//         None => {
//             ctx.reply("failed bypass").await?;
//         }
//     }

//     Ok(())
// }
