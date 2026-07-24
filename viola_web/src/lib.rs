use axum::{
    extract::Path,
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
};

#[derive(rust_embed::RustEmbed)]
#[folder = "dist/"]
pub struct Assets;

fn serve_file(path: &str) -> Response {
    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();

            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }

        None => (StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}

pub async fn index() -> impl IntoResponse {
    Html(String::from_utf8(Assets::get("index.html").unwrap().data.into()).unwrap())
}

pub async fn assets(Path(path): Path<String>) -> Response {
    if let Some(content) = Assets::get(&path) {
        let mime = mime_guess::from_path(&path).first_or_octet_stream();

        return ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response();
    }
    serve_file("index.html")
}

pub async fn asset(Path(path): Path<String>) -> Response {
    let path = format!("assets/{path}");

    match Assets::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();

            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn spa() -> impl IntoResponse {
    Html(String::from_utf8(Assets::get("index.html").unwrap().data.into()).unwrap())
}
