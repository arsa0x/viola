use axum::{Router, routing::get};
use viola_web::{assets, index, spa};

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async move {
        let app = Router::new()
            .route("/", get(index))
            .route("/{*path}", get(assets))
            .fallback(get(spa));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
            .await
            .unwrap();

        axum::serve(listener, app).await.unwrap();
    });
}
