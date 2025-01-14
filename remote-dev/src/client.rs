use std::path::Path;

use reqwest::{Response, StatusCode};

use crate::server::FileUploadRequest;

pub async fn client_main(url: String, file: &Path) {
    let response = upload_binary(url, file).await.unwrap();

    if response.status() == StatusCode::OK {
        println!("Success!");
    } else {
        println!("Failed!");
    }
}

async fn upload_binary(url: String, file: &Path) -> Result<Response, Box<dyn std::error::Error>> {
    // Using reqwest to demonstrate the client side
    let client = reqwest::Client::new();
    let bytes = std::fs::read(file)?;

    let metadata = FileUploadRequest {
        name: "example.dat".to_string(),
        bytes,
    };

    let binary_data = std::fs::read(file)?;

    Ok(client
        .post(&url)
        .json(&metadata)
        .body(binary_data)
        .send()
        .await?)
}
