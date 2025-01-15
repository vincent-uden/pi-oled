use std::{collections::HashMap, io::Write, path::PathBuf, process::Command};

use axum::{
    extract::Query,
    response::{IntoResponse, Response, Result},
    routing::{get, post},
    Json, Router,
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

pub async fn server_main(port: u16) {
    let app = Router::new()
        .route("/", get(root))
        .route("/upload", post(upload_binary))
        .route("/execute", post(execute_binary))
        .route("/kill", get(kill_pid));

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

#[derive(Deserialize, Serialize, Debug)]
pub struct ExecuteRequest {
    pub name: String,
    pub arguments: Vec<String>,
}

async fn execute_binary(Json(file_req): Json<ExecuteRequest>) -> Result<impl IntoResponse> {
    let child = Command::new(file_req.name)
        .args(file_req.arguments)
        .spawn()
        .map_err(|e| StatusCode::INTERNAL_SERVER_ERROR)?;

    println!("Spawned child process with pid: {}", child.id());
    let output = child
        .wait_with_output()
        .map_err(|e| StatusCode::INTERNAL_SERVER_ERROR)?;

    if output.status.success() {
        println!(
            "Child process ran successfully with output: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        Ok("Success!")
    } else {
        println!(
            "Child process failed with status code: {} and output: {}",
            output.status,
            String::from_utf8_lossy(&output.stdout)
        );
        Err(StatusCode::INTERNAL_SERVER_ERROR.into())
    }
}

#[axum::debug_handler]
async fn kill_pid(pid: Query<u32>) -> impl IntoResponse {
    let child = Command::new("kill").arg(pid.to_string()).spawn().unwrap();
    println!("Killed child process with pid: {}", child.id());
    let output = child.wait_with_output().unwrap();

    if output.status.success() {
        println!(
            "Child process killed successfully with output: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        "Success!"
    } else {
        println!(
            "Child process kill failed with status code: {} and output: {}",
            output.status,
            String::from_utf8_lossy(&output.stdout)
        );
        "Failed!"
    }
}
