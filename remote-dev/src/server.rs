use axum::{Router, routing::get};

pub async fn server_main(port: u16) {
    let app = Router::new().route("/", get(root));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> &'static str {
    "Hello, World!"
}
