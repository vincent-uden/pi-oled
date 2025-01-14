use std::{collections::HashMap, io::Write};

use axum::{
    body::Bytes,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

pub async fn server_main(port: u16) {
    let app = Router::new()
        .route("/", get(root))
        .route("/upload", post(upload_binary));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> &'static str {
    "Hello, World!"
}

#[derive(Deserialize, Serialize, Debug)]
pub struct FileUploadRequest {
    pub name: String,
    pub bytes: Vec<u8>,
}

async fn upload_binary(Json(file_req): Json<FileUploadRequest>) -> impl IntoResponse {
    let mut file = std::fs::File::create(file_req.name).unwrap();
    file.write_all(&file_req.bytes).unwrap();

    "File uploaded successfully!"
}
