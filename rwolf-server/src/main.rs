

use std::error::Error;
use std::sync::Arc;
use axum::extract::{Query, State};
use axum::{Json, Router};
use axum::http::StatusCode;
use axum::routing::post;
use chrono::{FixedOffset, Utc};
use futures::TryStreamExt;
use mongodb::bson::{doc, Bson, DateTime};
use mongodb::{Client, Collection};
use mongodb::bson::oid::ObjectId;
use mongodb::options::FindOptions;
use names::Name;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

const MONGO_URL: &str = "mongodb://127.0.0.1:27017/";
const POINT_TYPE: &str = "Point";
static TZ_UTC_PLUS8: Lazy<FixedOffset> = Lazy::new(||{
    FixedOffset::east_opt(8 * 3600).unwrap()
});

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
    let listener = tokio::net::TcpListener::bind("[::]:3000").await?;
    let app = Router::new()
        .route("/api/userposition", post(post_position).get(query_position))
        .fallback_service(
            ServeDir::new("vite-leaflet-demo/dist").append_index_html_on_directories(true)
        )
        .layer(TraceLayer::new_for_http())
        .with_state(AppState {
            collection,
            name_mutex: Arc::new(Mutex::new(()))
        });
    tracing::debug!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn post_position(
    State(state): State<AppState>,
    Json(input): Json<CreateUserPosition>,
) -> Result<Json<CreateUserPositionResp>, (StatusCode, String)> {
    let AppState { collection, name_mutex } = state;
    let CreateUserPosition {user, lng, lat} = input;
    match user {
        None => create_user_first_log(name_mutex, lng, lat, &collection).await,
        Some(u) => {
            let names = collection.distinct("user", doc! {}).await.map_err(internal_error)?;
            if names.iter().any(|b|b.as_str() == Some(&u)) {
                create_position(u, lng, lat, &collection).await
            } else {
                Err((StatusCode::BAD_REQUEST, format!("user not found - {}", u)))
            }

        },
    }
}

async fn create_position(username: String, lng: f64, lat: f64, collection: &Collection<UserPosition>) -> Result<Json<CreateUserPositionResp>, (StatusCode, String)> {
    let user_postiton = UserPosition {
        id: ObjectId::new(),
        user: username.clone(),
        create_at: DateTime::now(),
        location: GeoPoint::new(lng, lat),
    };
    let result = collection.insert_one(user_postiton).await.map_err(internal_error)?;
    let oid = result.inserted_id.as_object_id().ok_or_else(||(StatusCode::INTERNAL_SERVER_ERROR, "parse objectId error".to_string()))?;
    Ok(Json(CreateUserPositionResp {
        id: oid.to_hex(),
        user: username,
    }))
}

async fn create_user_first_log(mutex: Arc<Mutex<()>>, lng: f64, lat: f64, collection: &Collection<UserPosition>) -> Result<Json<CreateUserPositionResp>, (StatusCode, String)> {
    let _name_lock = mutex.lock().await;
    let names = collection.distinct("user", doc! {}).await.map_err(internal_error)?;
    let new_name = gen_name_loop(names)?;
    create_position(new_name, lng, lat, collection).await
}

fn gen_name_loop(names: Vec<Bson>) -> Result<String, (StatusCode, String)> {
    let mut generator_numbered = names::Generator::with_naming(Name::Numbered);
    loop {
        match generator_numbered.next() {
            None => break Err((StatusCode::INTERNAL_SERVER_ERROR, "generate name error".to_string())),
            Some(new_name) => {
                if names.iter().any(|b| b.as_str() == Some(&new_name)) {
                    tracing::warn!("create name dup occurred {}", new_name);
                    continue;
                } else {
                    break Ok(new_name)
                }
            }
        }
    }
}

async fn query_position(
    State(state): State<AppState>,
    Query(query): Query<QueryUserPosition>,
) -> Result<Json<Vec<QueryUserPositionResult>>, (StatusCode, String)> {
    let AppState { collection, name_mutex: _ } = state;
    let lng = query.lng;
    let lat = query.lat;
    let filter = doc! {
        "location": {
            "$nearSphere": {
                "$geometry": {
                    "type": "Point",
                    "coordinates": [lng, lat]
                },
                "$maxDistance": 20000,
            }
        }
    };
    let options = FindOptions::builder()
        .limit(1000)
        .build();

    let mut cursor = collection.find(filter).with_options(options).await.map_err(internal_error)?;
    let mut results = vec![];
    while let Some(rec) = cursor.try_next().await.map_err(internal_error)? {
        let UserPosition {
            id, user, create_at, location
        } = rec;
        let utc: chrono::DateTime<Utc> = create_at.to_system_time().into();

        let rec = QueryUserPositionResult {
            id: id.to_hex(),
            user,
            create_time: utc.with_timezone(&*TZ_UTC_PLUS8).format("%Y-%m-%d %H:%M:%S").to_string(),
            location: location.coordinates,
        };
        results.push(rec);
    }
    Ok(Json(results))
}

fn internal_error<E>(err: E) -> (StatusCode, String)
where E: Error {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

#[derive(Clone)]
struct AppState {
    collection: Collection<UserPosition>,
    name_mutex: Arc<Mutex<()>>
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
    user: Option<String>,
    lng: f64,
    lat: f64,
}

#[derive(Serialize)]
struct CreateUserPositionResp {
    id: String,
    user: String,
}

#[derive(Debug, Deserialize)]
struct QueryUserPosition {
    lng: f64,
    lat: f64,
}

#[derive(Serialize)]
struct QueryUserPositionResult {
    id: String,
    user: String,
    #[serde(rename = "createTime")]
    create_time: String,
    location: [f64; 2]
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use mongodb::{Client, Collection};
    use mongodb::bson::doc;
    use mongodb::bson::oid::ObjectId;
    use names::Name;
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
        let id = vec![
            ObjectId::parse_str("69418386e23fe4e12e20e60f").unwrap(),
            ObjectId::parse_str("6942d7cb5d25ecbd1e5243e7").unwrap(),
            ObjectId::parse_str("694a530eda0c2d3c6b50301e").unwrap(),
        ];
        for id in id {
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

    }

    #[tokio::test]
    async fn names_gen() {
        let mut generator = names::Generator::default();
        println!("{}", generator.next().unwrap());
        let mut generator_numbered = names::Generator::with_naming(Name::Numbered);
        println!("{}", generator_numbered.next().unwrap());
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
    // #[tokio::test]
    // async fn create_name_index() {
    //     let client = Client::with_uri_str(MONGO_URL).await.unwrap();
    //     let collection: Collection<UserPosition> = client.database("rwolf").collection("user_positions");
    //     let index = IndexModel::builder()
    //         .keys(doc!{"user": 1})
    //         .build();
    //     collection.create_index(index).await.unwrap();
    // }
}
