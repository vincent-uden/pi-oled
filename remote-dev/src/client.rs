use std::path::Path;

use reqwest::{Response, StatusCode};

use crate::{
    server::{ExecuteRequest, FileUploadRequest},
    ClientCommand,
};

pub async fn client_main(base_url: String, command: ClientCommand) {
    let resp = match command {
        ClientCommand::Upload { file } => {
            upload_binary(format!("{}/upload", base_url).to_string(), &file)
                .await
                .unwrap()
        }
        ClientCommand::Execute { file } => {
            execute_binary(format!("{}/execute", base_url).to_string(), &file)
                .await
                .unwrap()
        }
        ClientCommand::Kill { pid } => {
            kill_pid(format!("{}/kill/{}", base_url, pid).to_string(), pid)
                .await
                .unwrap()
        }
    };

    println!("Response: {:#?}", resp);
}

async fn upload_binary(url: String, file: &Path) -> Result<Response, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let bytes = std::fs::read(file)?;

    let metadata = FileUploadRequest {
        name: "example.dat".to_string(),
        bytes,
    };

    println!("Sending request to {} upload", url);
    println!("File name: {:#?}", metadata);

    Ok(client.post(&url).json(&metadata).send().await?)
}

async fn execute_binary(url: String, file: &Path) -> Result<Response, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let execute_request = ExecuteRequest {
        name: file.to_str().unwrap().to_string(),
        arguments: vec![],
    };

    println!("Sending request to execute {}", url);
    println!("File name: {:#?}", execute_request);

    Ok(client.post(&url).json(&execute_request).send().await?)
}

async fn kill_pid(url: String, pid: u32) -> Result<Response, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    Ok(client.post(format!("{}/{}", url, pid)).send().await?)
}
