

use std::error::Error;
use axum::extract::State;
use axum::{Json, Router};
use axum::http::StatusCode;
use axum::routing::{get, post};
use mongodb::bson::doc;
use mongodb::{Client, Collection};
use mongodb::bson::oid::ObjectId;
use mongodb::results::InsertOneResult;
use serde::{Deserialize, Serialize};
use tower_http::trace::TraceLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let db_connection = std::env::var("DATABASE_URL").unwrap_or_else(|_|"mongodb://127.0.0.1:27017/".to_string());
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
    let user_postiton = UserPosition {
        id: ObjectId::new(),
        user: input.user,
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
}

#[derive(Debug, Deserialize, Serialize)]
struct CreateUserPosition {
    user: String,
}