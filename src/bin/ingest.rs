// Ingestion-only binary.
// Run with:  cargo run --bin ingest
//
// Runs Met → AIC → Unsplash per query step, then exits.

use std::sync::{Arc, Mutex};
use venuscollect::{db, ingestion};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect("Failed to load .env file");
    println!("venuscollect — ingestion only");

    let conn = db::open().unwrap();
    let db   = Arc::new(Mutex::new(conn));

    ingestion::run_ingestion(db).await;
}
