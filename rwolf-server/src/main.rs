use std::error::Error;
use axum::Router;
use axum::routing::get;
use mongodb::bson::doc;
use mongodb::Client;
use tower_http::trace::TraceLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let db_connection = std::env::var("DATABASE_URL").unwrap_or_else(|_|"mongodb://127.0.0.1:27017/".to_string());
    let client = Client::with_uri_str(db_connection).await?;
    // pinging database
    client.database("rwolf").run_command(doc! {"ping": 1}).await?;
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_|format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()))
        .with(tracing_subscriber::fmt::layer())
        .init();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    let app = Router::new().route("/", get(||async { "hello".to_string() }))
        .layer(TraceLayer::new_for_http());
    tracing::debug!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
