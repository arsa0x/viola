use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;
use viola_core::{
    config::ParsedConfig,
    message::{
        interactive::carousel::{CarouselButton, CarouselCard},
        media::MediaSource,
    },
};
use whatsapp_rust::{
    anyhow::{self, anyhow},
    serde_json,
};

const NEPHI: &[u8] = include_bytes!("./nephi.jpg");

#[viola_macros::command(
  triggers = ["nekopoi", "neko", "kucing", "nkp"],
  category = "nsfw"
)]
async fn nekopoi(ctx: viola_core::Context) -> anyhow::Result<()> {
    let nekopoi = match Nekopoi::new(ctx.http_client.clone(), &ctx.config.parsed) {
        Ok(nekopoi) => nekopoi,
        Err(e) => return ctx.send().inapp_signup(e.to_string()).title("Failed").await,
    };

    let args = viola_core::Args::parse(
        &ctx.args,
        &[
            viola_core::args::flag_value(&["--search", "-s"]),
            viola_core::args::flag_value(&["--genre", "-g"]),
            viola_core::args::flag_value(&["--page", "-p"]),
            viola_core::args::flag_value(&["--id", "-i"]),
            viola_core::args::flag_value(&["--resolution", "-r"]),
        ],
    );

    if args.has("--search") {
        let Some(search) = args.value_parsed::<String>("--search") else {
            return ctx
                .send()
                .inapp_signup(format!(
                    "contoh penggunaan:\n> {}nekopoi --search query",
                    ctx.config.prefixes[0]
                ))
                .title("Viola")
                .await;
        };

        let page = args.value_parsed("--page").unwrap_or(1);
        let s = nekopoi.search_by_query(&search, page).await?;

        let Some(results) = s.result else {
            return ctx
                .send()
                .inapp_signup("gk ketemu ngab")
                .title("Viola")
                .quoted()
                .await;
        };

        println!("{:#?}", results);

        let cards = results.into_iter().map(|result| {
            let card = CarouselCard::new(format!("title: {}\nid: {}", result.title, result.id))
                .footer(result.date)
                .button(CarouselButton::QuickReply {
                    display_text: "Select".into(),
                    id: format!("{}nekopoi --id {}", ctx.config.prefixes[0], result.id),
                });

            match result.image {
                ImageField::Url(url) => card.header_image(MediaSource::Url(url)),
                ImageField::Bool(_) => card.header_image(MediaSource::Bytes(NEPHI.to_vec())),
            }
        });

        return ctx
            .send()
            .carousel(format!(
                "query: {}\ntotal: {}\npage: {}\ntotal page: {}",
                search, s.total, page, s.total_pages
            ))
            .cards(cards)
            .quoted()
            .await;
    } else if args.has("--genre") {
        let Some(genre) = args.value_parsed::<String>("--genre") else {
            return ctx
                .send()
                .inapp_signup(format!(
                    "contoh penggunaan:\n> {}nekopoi --genre loli",
                    ctx.config.prefixes[0]
                ))
                .title("Viola")
                .await;
        };
        let page = args.value_parsed("--page").unwrap_or(1);
        let s = nekopoi.search_by_genre(&[&genre]).await?;

        let Some(results) = s.result else {
            return ctx
                .send()
                .inapp_signup("gk ketemu woilah")
                .title("Viola")
                .quoted()
                .await;
        };

        let cards = results.into_iter().map(|result| {
            let card = CarouselCard::new(format!("title: {}\nid: {}", result.title, result.id))
                .footer(result.date)
                .button(CarouselButton::QuickReply {
                    display_text: "Select".into(),
                    id: format!("{}nekopoi --id {}", ctx.config.prefixes[0], result.id),
                });
            match result.image {
                ImageField::Url(url) => card.header_image(MediaSource::Url(url)),
                ImageField::Bool(_) => card.header_image(MediaSource::Bytes(NEPHI.to_vec())),
            }
        });

        return ctx
            .send()
            .carousel(format!(
                "query: {}\ntotal: {}\npage: {}\ntotal page: {}",
                genre, s.total, page, s.total_pages
            ))
            .cards(cards)
            .quoted()
            .await;
    } else {
        let Some(id) = args.value_parsed::<u32>("--id") else {
            return ctx
                .send()
                .inapp_signup("id nya mana cik")
                .title("Viola")
                .quoted()
                .await;
        };

        let post = nekopoi.post(id).await?;

        let image = match &post.image {
            ImageField::Bool(_) => MediaSource::Bytes(NEPHI.to_vec()),
            ImageField::Url(url) => MediaSource::Url(url.to_string()),
        };

        ctx.send()
            .image(image)
            .caption(format!("{:#?}", post))
            .quoted()
            .await
    }
}

struct Nekopoi<'a> {
    base: &'a str,
    headers: HeaderMap,
    client: reqwest::Client,
}

impl<'a> Nekopoi<'a> {
    pub fn new(client: reqwest::Client, parsed: &'a ParsedConfig) -> anyhow::Result<Self> {
        let Some(base) = parsed.get("nekopoi_base") else {
            return Err(anyhow!("Missing config: nekopoi_base"));
        };

        let Some(build_code) = parsed.get("nekopoi_build_code") else {
            return Err(anyhow!("Missing config: nekopoi_build_code"));
        };

        let Some(signature) = parsed.get("nekopoi_signature") else {
            return Err(anyhow!("Missing config: nekopoi_signature"));
        };

        let Some(user_agent) = parsed.get("nekopoi_user_agent") else {
            return Err(anyhow!("Missing config: nekopoi_user_agent"));
        };

        let Some(token) = parsed.get("nekopoi_token") else {
            return Err(anyhow!("Missing config: nekopoi_token"));
        };

        let mut headers = HeaderMap::new();

        headers.insert(
            reqwest::header::USER_AGENT,
            HeaderValue::from_str(user_agent)?,
        );

        headers.insert("AppBuildCode", HeaderValue::from_str(build_code)?);

        headers.insert("AppSignature", HeaderValue::from_str(signature)?);

        headers.insert("Token", HeaderValue::from_str(token)?);

        headers.insert(
            reqwest::header::ACCEPT,
            HeaderValue::from_static("application/json"),
        );

        Ok(Nekopoi {
            headers,
            base,
            client,
        })
    }

    async fn get(
        &self,
        path: &str,
        query: Option<&[(&str, &str)]>,
    ) -> anyhow::Result<reqwest::Response> {
        let mut req = self
            .client
            .get(format!("{}{}", self.base, path))
            .headers(self.headers.clone());

        if let Some(query) = query {
            req = req.query(query);
        }

        Ok(req.send().await?)
    }

    /// @f("all")
    /// Object c(@t("page") int i10, @t("category") String str, InterfaceC1135d<? super All> interfaceC1135d);
    ///
    /// [
    ///   { "label": "Hentai", "id": 2 },
    ///   { "label": "JAV", "id": 4 },
    ///   { "label": "3D Hentai", "id": 81 },
    ///   { "label": "2D Animation", "id": 682 },
    ///   { "label": "JAV Cosplay", "id": 1 }
    /// ]
    pub async fn all(&self, category: &str, page: &str) -> anyhow::Result<All> {
        Ok(self
            .get("/all", Some(&[("page", page), ("category", category)]))
            .await?
            .json::<All>()
            .await?)
    }

    /// @f("series")
    /// Object g(@t("id") int i10, InterfaceC1135d<? super Series> interfaceC1135d);
    pub async fn series(&self, id: u32) -> anyhow::Result<serde_json::Value> {
        Ok(self
            .get("/series", Some(&[("id", &id.to_string())]))
            .await?
            .json::<serde_json::Value>()
            .await?)
    }

    /// @f("genre")
    /// Object a(InterfaceC1135d<? super SearchListGenres> interfaceC1135d);
    pub async fn get_genres(&self) -> anyhow::Result<SearchListGenres> {
        Ok(self
            .get("/genre", None)
            .await?
            .json::<SearchListGenres>()
            .await?)
    }

    /// @f("searchByGenre")
    /// Object f(@t("term") List<Integer> list, InterfaceC1135d<? super SearchByGenre> interfaceC1135d);
    ///
    /// [
    ///   { "id": 72, "name": "Action" },
    ///   { "id": 48, "name": "Ahegao" },
    ///   { "id": 39, "name": "Anal" },
    ///   { "id": 529, "name": "Armpit" },
    ///   { "id": 40, "name": "BDSM" },
    ///   { "id": 33, "name": "Big Oppai" },
    ///   { "id": 684, "name": "Blackmail" },
    ///   { "id": 633, "name": "Blonde" },
    ///   { "id": 30, "name": "Blowjob" },
    ///   { "id": 58, "name": "Bondage" },
    ///   { "id": 686, "name": "Cheating" },
    ///   { "id": 244, "name": "Comedy" },
    ///   { "id": 31, "name": "Creampie" },
    ///   { "id": 517, "name": "Dark Skin" },
    ///   { "id": 546, "name": "DILF" },
    ///   { "id": 73, "name": "Elf" },
    ///   { "id": 585, "name": "Exhibitionist" },
    ///   { "id": 56, "name": "Fellatio" },
    ///   { "id": 584, "name": "Female Monster" },
    ///   { "id": 61, "name": "Femdom" },
    ///   { "id": 52, "name": "Footjob" },
    ///   { "id": 35, "name": "Forced" },
    ///   { "id": 641, "name": "Furry" },
    ///   { "id": 50, "name": "Futanari" },
    ///   { "id": 55, "name": "Gangbang" },
    ///   { "id": 70, "name": "Gore" },
    ///   { "id": 687, "name": "Gyaru" },
    ///   { "id": 681, "name": "Handjob" },
    ///   { "id": 43, "name": "Harem" },
    ///   { "id": 683, "name": "Horror" },
    ///   { "id": 32, "name": "Housewife" },
    ///   { "id": 678, "name": "Humilation" },
    ///   { "id": 117, "name": "Humiliation" },
    ///   { "id": 530, "name": "Hypnotize" },
    ///   { "id": 46, "name": "Incest" },
    ///   { "id": 532, "name": "Intercrural" },
    ///   { "id": 679, "name": "JAV" },
    ///   { "id": 255, "name": "Lactation" },
    ///   { "id": 36, "name": "Loli" },
    ///   { "id": 49, "name": "Maid" },
    ///   { "id": 583, "name": "Male Monster" },
    ///   { "id": 29, "name": "Masturbation" },
    ///   { "id": 59, "name": "Megane" },
    ///   { "id": 28, "name": "MILF" },
    ///   { "id": 573, "name": "Mind Control" },
    ///   { "id": 47, "name": "Monster" },
    ///   { "id": 27, "name": "Netorare" },
    ///   { "id": 71, "name": "Nurse" },
    ///   { "id": 548, "name": "Old man" },
    ///   { "id": 544, "name": "Onee-san" },
    ///   { "id": 44, "name": "Oral" },
    ///   { "id": 38, "name": "Paizuri" },
    ///   { "id": 54, "name": "Pantyhose" },
    ///   { "id": 67, "name": "Pregnant" },
    ///   { "id": 675, "name": "Prostitution" },
    ///   { "id": 51, "name": "Rape" },
    ///   { "id": 41, "name": "Romance" },
    ///   { "id": 615, "name": "Saimin" },
    ///   { "id": 37, "name": "Schoolgirl" },
    ///   { "id": 672, "name": "Semi-Hentai" },
    ///   { "id": 674, "name": "Sex Toys" },
    ///   { "id": 65, "name": "Shibari" },
    ///   { "id": 212, "name": "Shota" },
    ///   { "id": 62, "name": "Stocking" },
    ///   { "id": 506, "name": "Succubus" },
    ///   { "id": 60, "name": "Supranatural" },
    ///   { "id": 66, "name": "Swimsuit" },
    ///   { "id": 42, "name": "Tentacles" },
    ///   { "id": 498, "name": "Threesome" },
    ///   { "id": 53, "name": "Tsundere" },
    ///   { "id": 685, "name": "Ugly Bastard" },
    ///   { "id": 69, "name": "Uncensored" },
    ///   { "id": 57, "name": "Vanilla" },
    ///   { "id": 34, "name": "Virgin" },
    ///   { "id": 180, "name": "Yaoi" },
    ///   { "id": 45, "name": "Yuri" }
    /// ]
    pub async fn search_by_genre(&self, genres: &[&str]) -> anyhow::Result<SearchByGenre> {
        let mut g = Vec::new();
        for genre in genres {
            g.push(("term", *genre));
        }

        Ok(self
            .get("/searchByGenre", Some(g.as_slice()))
            .await?
            .json::<SearchByGenre>()
            .await?)
    }

    /// @f("search")
    /// Object b(@t("q") String str, @t("page") int i10, InterfaceC1135d<? super Search> interfaceC1135d);
    pub async fn search_by_query(&self, query: &str, page: u8) -> anyhow::Result<Search> {
        Ok(self
            .get(
                "/search",
                Some(&[("q", query), ("page", &page.to_string())]),
            )
            .await?
            .json::<Search>()
            .await?)
    }

    /// @f("recent")
    /// Object i(InterfaceC1135d<? super Recent> interfaceC1135d);
    pub async fn recent(&self) -> anyhow::Result<Recent> {
        Ok(self.get("/recent", None).await?.json::<Recent>().await?)
    }

    /// @f("listall")
    /// Object j(@t("letter") String str, @t(k.EVENT_TYPE_KEY) String str2, @t("page") int i10, InterfaceC1135d<? super GetList> interfaceC1135d);
    pub async fn list_all(&self, letter: &str, page: u8) -> anyhow::Result<GetList> {
        Ok(self
            .get(
                "/listall",
                Some(&[("letter", letter), ("page", &page.to_string())]),
            )
            .await?
            .json::<GetList>()
            .await?)
    }

    /// @f("post")
    /// Object k(@t("id") int i10, InterfaceC1135d<? super Posts> interfaceC1135d);
    pub async fn post(&self, id: u32) -> anyhow::Result<Posts> {
        Ok(self
            .get("/post", Some(&[("id", &id.to_string())]))
            .await?
            .json::<Posts>()
            .await?)
    }
}

pub struct Genre {
    pub id: u32,
    pub name: &'static str,
}

pub const GENRES: [Genre; 76] = [
    Genre {
        id: 72,
        name: "Action",
    },
    Genre {
        id: 48,
        name: "Ahegao",
    },
    Genre {
        id: 39,
        name: "Anal",
    },
    Genre {
        id: 529,
        name: "Armpit",
    },
    Genre {
        id: 40,
        name: "BDSM",
    },
    Genre {
        id: 33,
        name: "Big Oppai",
    },
    Genre {
        id: 684,
        name: "Blackmail",
    },
    Genre {
        id: 633,
        name: "Blonde",
    },
    Genre {
        id: 30,
        name: "Blowjob",
    },
    Genre {
        id: 58,
        name: "Bondage",
    },
    Genre {
        id: 686,
        name: "Cheating",
    },
    Genre {
        id: 244,
        name: "Comedy",
    },
    Genre {
        id: 31,
        name: "Creampie",
    },
    Genre {
        id: 517,
        name: "Dark Skin",
    },
    Genre {
        id: 546,
        name: "DILF",
    },
    Genre {
        id: 73,
        name: "Elf",
    },
    Genre {
        id: 585,
        name: "Exhibitionist",
    },
    Genre {
        id: 56,
        name: "Fellatio",
    },
    Genre {
        id: 584,
        name: "Female Monster",
    },
    Genre {
        id: 61,
        name: "Femdom",
    },
    Genre {
        id: 52,
        name: "Footjob",
    },
    Genre {
        id: 35,
        name: "Forced",
    },
    Genre {
        id: 641,
        name: "Furry",
    },
    Genre {
        id: 50,
        name: "Futanari",
    },
    Genre {
        id: 55,
        name: "Gangbang",
    },
    Genre {
        id: 70,
        name: "Gore",
    },
    Genre {
        id: 687,
        name: "Gyaru",
    },
    Genre {
        id: 681,
        name: "Handjob",
    },
    Genre {
        id: 43,
        name: "Harem",
    },
    Genre {
        id: 683,
        name: "Horror",
    },
    Genre {
        id: 32,
        name: "Housewife",
    },
    Genre {
        id: 678,
        name: "Humilation",
    },
    Genre {
        id: 117,
        name: "Humiliation",
    },
    Genre {
        id: 530,
        name: "Hypnotize",
    },
    Genre {
        id: 46,
        name: "Incest",
    },
    Genre {
        id: 532,
        name: "Intercrural",
    },
    Genre {
        id: 679,
        name: "JAV",
    },
    Genre {
        id: 255,
        name: "Lactation",
    },
    Genre {
        id: 36,
        name: "Loli",
    },
    Genre {
        id: 49,
        name: "Maid",
    },
    Genre {
        id: 583,
        name: "Male Monster",
    },
    Genre {
        id: 29,
        name: "Masturbation",
    },
    Genre {
        id: 59,
        name: "Megane",
    },
    Genre {
        id: 28,
        name: "MILF",
    },
    Genre {
        id: 573,
        name: "Mind Control",
    },
    Genre {
        id: 47,
        name: "Monster",
    },
    Genre {
        id: 27,
        name: "Netorare",
    },
    Genre {
        id: 71,
        name: "Nurse",
    },
    Genre {
        id: 548,
        name: "Old man",
    },
    Genre {
        id: 544,
        name: "Onee-san",
    },
    Genre {
        id: 44,
        name: "Oral",
    },
    Genre {
        id: 38,
        name: "Paizuri",
    },
    Genre {
        id: 54,
        name: "Pantyhose",
    },
    Genre {
        id: 67,
        name: "Pregnant",
    },
    Genre {
        id: 675,
        name: "Prostitution",
    },
    Genre {
        id: 51,
        name: "Rape",
    },
    Genre {
        id: 41,
        name: "Romance",
    },
    Genre {
        id: 615,
        name: "Saimin",
    },
    Genre {
        id: 37,
        name: "Schoolgirl",
    },
    Genre {
        id: 672,
        name: "Semi-Hentai",
    },
    Genre {
        id: 674,
        name: "Sex Toys",
    },
    Genre {
        id: 65,
        name: "Shibari",
    },
    Genre {
        id: 212,
        name: "Shota",
    },
    Genre {
        id: 62,
        name: "Stocking",
    },
    Genre {
        id: 506,
        name: "Succubus",
    },
    Genre {
        id: 60,
        name: "Supranatural",
    },
    Genre {
        id: 66,
        name: "Swimsuit",
    },
    Genre {
        id: 42,
        name: "Tentacles",
    },
    Genre {
        id: 498,
        name: "Threesome",
    },
    Genre {
        id: 53,
        name: "Tsundere",
    },
    Genre {
        id: 685,
        name: "Ugly Bastard",
    },
    Genre {
        id: 69,
        name: "Uncensored",
    },
    Genre {
        id: 57,
        name: "Vanilla",
    },
    Genre {
        id: 34,
        name: "Virgin",
    },
    Genre {
        id: 180,
        name: "Yaoi",
    },
    Genre {
        id: 45,
        name: "Yuri",
    },
];

#[derive(Debug, Deserialize)]
pub struct PostsSeries {
    pub content: String,
    pub genre: String,
    pub id: u32,
    pub image: ImageField,
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct DisqusCommentsResultMessage {
    pub raw: String,
}

#[derive(Debug, Deserialize)]
pub struct NewRelease {
    pub result: String,
}

#[derive(Debug, Deserialize)]
pub struct Posts {
    pub content: String,
    pub date: String,
    pub download: Vec<PostsDownload>,
    pub id: u32,
    pub image: ImageField,
    pub note: String,
    pub series: PostsSeries,
    pub slug: Option<String>,
    pub stream: Vec<PostsStream>,
    #[serde(rename = "streamnote")]
    pub stream_note: String,
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct GetList {
    pub result: Vec<GetListResult>,
    pub total: u32,
    #[serde(rename = "totalPages")]
    pub total_pages: u32,
}

#[derive(Debug, Deserialize)]
pub struct All {
    pub result: Vec<AllListResult>,
    pub total: u32,
    #[serde(rename = "totalPages")]
    pub total_pages: u32,
}

#[derive(Debug, Deserialize)]
pub struct SearchListGenres {
    pub total: u32,
    pub data: Vec<SearchListGenresData>,
}

#[derive(Debug, Deserialize)]
pub struct Recent {
    pub carousel: Vec<RecentItem>,
    pub posts: Vec<RecentPost>,
}

#[derive(Debug, Deserialize)]
pub struct SearchListGenresData {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct SeriesEpisode {
    pub date: String,
    pub id: u32,
    pub image: ImageField,
    pub slug: Option<String>,
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct RecentItem {
    pub description: String,
    pub id: u32,
    pub image: ImageField,
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct Series {
    #[serde(rename = "episode")]
    pub list_episode: Option<Vec<SeriesEpisode>>,
    pub date: String,
    pub description: String,
    pub id: u32,
    pub image: ImageField,
    pub info_meta: SeriesInfoMeta,
    pub title: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum ImageField {
    Url(String),
    Bool(bool),
}

#[derive(Debug, Deserialize)]
pub struct SearchResult {
    pub date: String,
    pub id: u32,
    pub image: ImageField,
    pub slug: Option<String>,
    pub title: String,
    #[serde(rename = "type")]
    pub content_type: String,
}

#[derive(Debug, Deserialize)]
pub struct DisqusCommentsResult {
    pub author: DisqusCommentsResultAuthor,
    pub created_at: String,
    pub id: String,
    pub like: DisqusCommentsResultLikes,
    pub message: DisqusCommentsResultMessage,
}

#[derive(Debug, Deserialize)]
pub struct Search {
    pub result: Option<Vec<SearchResult>>,
    pub total: u32,
    #[serde(rename = "totalPages")]
    pub total_pages: u32,
}

#[derive(Debug, Deserialize)]
pub struct PostsDownloadLinks {
    pub link: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchByGenre {
    pub result: Option<Vec<SearchByGenreResult>>,
    pub total: u32,
    #[serde(rename = "totalPages")]
    pub total_pages: u32,
}

#[derive(Debug, Deserialize)]
pub struct SearchByGenreResult {
    pub date: String,
    pub id: u32,
    pub image: ImageField,
    pub link: Option<String>,
    pub slug: Option<String>,
    pub title: String,
    #[serde(rename = "type")]
    pub content_type: String,
}

#[derive(Debug, Deserialize)]
pub struct NewReleaseItem {
    pub episode: String,
    pub image: ImageField,
    pub release_date: String,
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct DisqusCommentsResultAuthor {
    pub avatar: String,
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct PostsDownload {
    pub links: Vec<PostsDownloadLinks>,
}

#[derive(Debug, Deserialize)]
pub struct GetListResult {
    pub date: String,
    pub id: u32,
    pub image: ImageField,
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct AllListResult {
    pub date: String,
    pub id: u32,
    pub image: ImageField,
    pub link: Option<String>,
    pub slug: Option<String>,
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct Updater {
    pub ad_filter_url: String,
    pub ad_main_url1: String,
    pub ad_main_url2: String,
    #[serde(rename = "adEnabled")]
    pub bda: String,
    pub latest_version: String,
    pub latest_version_code: u32,
    pub one_li: String,
    pub release_notes: Vec<String>,
    pub update_url: String,
}

#[derive(Debug, Deserialize)]
pub struct DisqusComments {
    pub has_next: bool,
    pub has_prev: bool,
    pub result: Vec<DisqusCommentsResult>,
    pub thread: String,
    pub total: u32,
}

#[derive(Debug, Deserialize)]
pub struct RecentPost {
    pub category: String,
}

#[derive(Debug, Deserialize)]
pub struct PostsStream {
    pub link: String,
}

#[derive(Debug, Deserialize)]
pub struct DisqusCommentsResultLikes {
    pub dislikes: u32,
    pub likes: u32,
}

#[derive(Debug, Deserialize)]
pub struct SeriesInfoMetaGenre {
    pub slug: Option<String>,
    pub term_id: u32,
}

#[derive(Debug, Deserialize)]
pub struct SeriesInfoMeta {
    pub aliases: String,
    pub durasi: String,
    pub episode: String,
    pub genre: Option<Vec<SeriesInfoMetaGenre>>,
    pub produser: String,
    pub skor: String,
    pub status: String,
    pub tayang: String,
}
