use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    time::Duration,
};

use reqwest::{Response, StatusCode};

use crate::{
    server::{ExecuteRequest, FileUploadRequest},
    ClientCommand,
};

pub async fn client_main(
    base_url: String,
    command: ClientCommand,
) -> Result<String, Box<dyn std::error::Error>> {
    let resp = match command {
        ClientCommand::Upload { file } => {
            upload_binary(format!("{}/upload", base_url).to_string(), &file).await?
        }
        ClientCommand::Execute { file } => {
            execute_binary(format!("{}/execute", base_url).to_string(), &file).await?
        }
        ClientCommand::Kill { pid } => {
            kill_pid(format!("{}/kill/{}", base_url, pid).to_string(), pid).await?
        }
        ClientCommand::Run { file } => {
            let run_path = PathBuf::from(&format!(
                "./{}",
                file.file_name().unwrap().to_str().unwrap()
            ));
            upload_binary(format!("{}/upload", base_url).to_string(), &file).await?;
            execute_binary(format!("{}/execute", base_url).to_string(), &run_path).await?
        }
    };

    println!("Response: {:#?}", resp);

    Ok("Success!".to_string())
}

async fn upload_binary(url: String, file: &Path) -> Result<Response, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let bytes = std::fs::read(file)?;

    let metadata = FileUploadRequest {
        name: file
            .file_name()
            .unwrap_or(OsStr::new("uploaded_file"))
            .to_string_lossy()
            .to_string(),
        bytes,
    };

    println!("Sending request to {} upload", url);
    println!("File name: {:#?}", metadata.name);

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
    Ok(client
        .post(format!("{}/{}", url, pid))
        .timeout(Duration::from_secs(5))
        .send()
        .await?)
}
