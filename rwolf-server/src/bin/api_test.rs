use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct CreateUserPosition {
    user: String,
    lng: f64,
    lat: f64,
}

#[tokio::main]
async fn main() {
    let client = Client::new();
    let body = CreateUserPosition {
        user: "mike".to_string(),
        lng: 116.3527318941201,
        lat: 39.950800621620495,
    };
    let response = client.post("http://127.0.0.1:3000/api/userposition")
        .json(&body)
        .send().await;
    match response {
        Ok(response) => {println!("{:?}", response);},
        Err(err) => {println!("error: {}", err);},
    }

}