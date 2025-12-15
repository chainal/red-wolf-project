use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct CreateUserPosition {
    user: String,
}

#[tokio::main]
async fn main() {
    let client = Client::new();
    let body = CreateUserPosition {
        user: "liuchuang".to_string(),
    };
    let response = client.post("http://127.0.0.1:3000/api/userposition")
        .json(&body)
        .send().await.unwrap();
    println!("{:?}", response)
}