

use std::error::Error;
use axum::extract::State;
use axum::{Json, Router};
use axum::http::StatusCode;
use axum::routing::{get, post};
use mongodb::bson::{doc, DateTime};
use mongodb::{Client, Collection};
use mongodb::bson::oid::ObjectId;
use mongodb::results::InsertOneResult;
use serde::{Deserialize, Serialize};
use tower_http::trace::TraceLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

const MONGO_URL: &str = "mongodb://127.0.0.1:27017/";
const POINT_TYPE: &str = "Point";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let db_connection = std::env::var("DATABASE_URL").unwrap_or_else(|_|MONGO_URL.to_string());
    let client = Client::with_uri_str(db_connection).await?;
    // pinging database
    client.database("rwolf").run_command(doc! {"ping": 1}).await?;
    let collection: Collection<UserPosition> = client.database("rwolf").collection("user_positions");

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_|format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()))
        .with(tracing_subscriber::fmt::layer())
        .init();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    let app = Router::new()
        .route("/", get(||async { "hello".to_string() }))
        .route("/api/userposition", post(post_position))
        .layer(TraceLayer::new_for_http())
        .with_state(collection);
    tracing::debug!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn post_position(
    State(db): State<Collection<UserPosition>>,
    Json(input): Json<CreateUserPosition>,
) -> Result<Json<InsertOneResult>, (StatusCode, String)> {
    let datetime = DateTime::now();
    let user_postiton = UserPosition {
        id: ObjectId::new(),
        user: input.user,
        create_at: datetime,
        location: GeoPoint::new(input.lng, input.lat),
    };
    let result = db.insert_one(user_postiton).await.map_err(internal_error)?;
    Ok(Json(result))
}

fn internal_error<E>(err: E) -> (StatusCode, String)
where E: Error {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}


#[derive(Debug, Serialize, Deserialize)]
struct UserPosition {
    #[serde(rename = "_id")]
    id: ObjectId,
    user: String,
    #[serde(rename = "createtime")]
    create_at: DateTime,
    location: GeoPoint,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeoPoint {
    #[serde(rename = "type")]
    geo_type: String,
    coordinates: [f64; 2],
}

impl GeoPoint {
    fn new(lng: f64, lat: f64) -> Self {
        Self {
            geo_type: POINT_TYPE.to_string(),
            coordinates: [lng, lat],
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct CreateUserPosition {
    user: String,
    lng: f64,
    lat: f64,
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use mongodb::{Client, Collection};
    use mongodb::bson::doc;
    use mongodb::bson::oid::ObjectId;
    use crate::{UserPosition, MONGO_URL};

    #[tokio::test]
    async fn list_records() {
        let client = Client::with_uri_str(MONGO_URL).await.unwrap();
        let collection: Collection<UserPosition> = client.database("rwolf").collection("user_positions");
        let num = collection.count_documents(doc! {}).await;
        if let Ok(num) = num {
            println!("total {} documents", num);
        }
        let mut cursor = collection.find(doc! {}).await.unwrap();
        let mut scan_num = 0;
        while let Some(result) = cursor.next().await {
            if scan_num >= 1000 {
                break;
            }
            if let Ok(result) = result {
                println!("{:?}", result);
            }
            scan_num += 1;
        }
    }

    #[tokio::test]
    async fn delete_records() {
        let client = Client::with_uri_str(MONGO_URL).await.unwrap();
        let collection: Collection<UserPosition> = client.database("rwolf").collection("user_positions");
        let id = ObjectId::parse_str("69417f7df39c2647a36fb0a7").unwrap();
        let result = collection.delete_one(doc! {"_id": id}).await;
        match result {
            Ok(result) => {
                println!("delete success {:?}", result);
            },
            Err(e) => {
                println!("delete error {}", e);
            }
        }
    }

    // DO NOT DELETE
    // #[tokio::test]
    // async fn create_geometry_index() {
    //     let client = Client::with_uri_str(MONGO_URL).await.unwrap();
    //     let collection: Collection<UserPosition> = client.database("rwolf").collection("user_positions");
    //     let index = IndexModel::builder()
    //         .keys(doc! {"location": "2dsphere"})
    //         .build();
    //     collection.create_index(index).await.unwrap();
    // }
}
