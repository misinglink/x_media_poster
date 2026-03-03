// Combined runner — ingestion + scheduler + UI all in one process.
// For independent operation use:
//   cargo run --bin ingest     (ingestion only)
//   cargo run --bin scheduler  (scheduler + UI only)

use std::sync::{Arc, Mutex};
use venuscollect::{db, ingestion, scheduler, ui, TWEET_INTERVAL_SECS};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect("Failed to load .env file");
    println!("venuscollect v0.1.0 — combined mode");

    let conn = db::open().unwrap();
    let db   = Arc::new(Mutex::new(conn));

    // Interleaved ingestion: Met → AIC → Unsplash per query step
    tokio::spawn(ingestion::run_ingestion(db.clone()));

    tokio::spawn(scheduler::run_tweet_scheduler(db.clone(), TWEET_INTERVAL_SECS));

    ui::serve(db).await;
}
