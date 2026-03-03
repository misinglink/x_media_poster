// Scheduler + UI binary — no ingestion.
// Run with:  cargo run --bin scheduler
//
// Serves the review UI and posts one approved artwork to X every hour.
// Run `cargo run --bin ingest` separately (or on a cron) to keep the DB fresh.

use std::sync::{Arc, Mutex};
use venuscollect::{db, scheduler, ui, TWEET_INTERVAL_SECS};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect("Failed to load .env file");
    println!("venuscollect — scheduler + UI");

    let conn = db::open().unwrap();
    let db   = Arc::new(Mutex::new(conn));

    tokio::spawn(scheduler::run_tweet_scheduler(db.clone(), TWEET_INTERVAL_SECS));

    ui::serve(db).await;
}
