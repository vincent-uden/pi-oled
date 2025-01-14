use reqwest::StatusCode;

pub async fn client_main(url: String) {
    let client = reqwest::Client::new();
    let response = client.get(url).send().await.unwrap();

    if response.status() == StatusCode::OK {
        println!("Success!");
    } else {
        println!("Failed!");
    }
}
