pub mod aic;
pub mod met;
pub mod unsplash;

use duckdb::Connection;
use reqwest::Client;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;

use crate::db;

/// Download raw image bytes from any URL.
pub async fn download_image(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let client = Client::new();
    let bytes = client.get(url).send().await?.bytes().await?;
    Ok(bytes.to_vec())
}

/// Run a full ingestion pass, interleaving all three sources query-by-query:
///
///   step 1 → Met[0]  + AIC[0]  + Unsplash[0]
///   step 2 → Met[1]  + AIC[1]  + Unsplash[1]
///   ...
///
/// QUERIES (museum-tuned) drives Met + AIC.
/// UNSPLASH_QUERIES (photo-tuned) drives Unsplash.
/// The loop runs until the longer list is exhausted.
///
/// The natural time Met + AIC take per step acts as free pacing between
/// Unsplash calls. An additional `UNSPLASH_INTER_QUERY_SLEEP`-second sleep
/// is added after each Unsplash call to stay under the 50 req/hour demo cap.
pub async fn run_ingestion(db_arc: Arc<Mutex<Connection>>) {
    let unsplash_key = std::env::var("UNSPLASH_ACCESS_KEY")
        .expect("UNSPLASH_ACCESS_KEY not set in .env");

    let museum_queries   = crate::QUERIES;
    let unsplash_queries = crate::UNSPLASH_QUERIES;
    let steps = museum_queries.len().max(unsplash_queries.len());

    println!(">>> ingestion started ({steps} steps)");
    let mut unsplash_remaining = 50u32;

    for i in 0..steps {
        println!("\n=== step {}/{steps} ===", i + 1);

        // ── Met Museum ───────────────────────────────────────────────────────
        if let Some(query) = museum_queries.get(i) {
            match met::search_met(query).await {
                Ok(ids) => {
                    println!("Met [{query}]: {} results", ids.len());
                    for id in ids {
                        let object_id = format!("met_{id}");
                        {
                            let conn = db_arc.lock().unwrap();
                            if db::artwork_exists(&conn, &object_id).unwrap_or(false) {
                                continue;
                            }
                        }
                        if let Ok(Some(art)) = met::fetch_met_artwork(id).await {
                            let conn = db_arc.lock().unwrap();
                            if let Err(e) = db::insert_artwork(
                                &conn, &object_id, &art.title, &art.artist,
                                &art.date, &art.image_url, &art.met_url,
                                "MetMuseum", query, None,
                            ) {
                                println!("  Met DB error: {e}");
                            }
                        }
                    }
                }
                Err(e) => println!("Met error: {e}"),
            }
        }

        // ── Art Institute of Chicago ─────────────────────────────────────────
        if let Some(query) = museum_queries.get(i) {
            match aic::search_aic(query).await {
                Ok(items) => {
                    println!("AIC [{query}]: {} results", items.len());
                    for item in items {
                        let object_id = format!("aic_{}", item.id);
                        {
                            let conn = db_arc.lock().unwrap();
                            if db::artwork_exists(&conn, &object_id).unwrap_or(false) {
                                continue;
                            }
                        }
                        if let Ok(Some(art)) = aic::fetch_aic_artwork(item.id).await {
                            if let Some(ref image_id) = art.image_id {
                                let image_url  = aic::aic_image_url(image_id);
                                let source_url = format!("https://www.artic.edu/artworks/{}", art.id);
                                let conn = db_arc.lock().unwrap();
                                if let Err(e) = db::insert_artwork(
                                    &conn, &object_id, &art.title, &art.artist,
                                    &art.date, &image_url, &source_url,
                                    "AIC", query, item.score,
                                ) {
                                    println!("  AIC DB error: {e}");
                                }
                            }
                        }
                    }
                }
                Err(e) => println!("AIC error: {e}"),
            }
        }

        // ── Unsplash ─────────────────────────────────────────────────────────
        if let Some(query) = unsplash_queries.get(i) {
            if unsplash_remaining < 3 {
                println!("Unsplash: quota exhausted — skipping remaining queries");
            } else {
                match unsplash::search_unsplash(query, &unsplash_key, crate::UNSPLASH_MAX_PAGES).await {
                    Ok((photos, remaining)) => {
                        unsplash_remaining = remaining;
                        let mut inserted = 0usize;
                        let mut skipped  = 0usize;
                        for photo in &photos {
                            let object_id = format!("unsplash_{}", photo.id);
                            {
                                let conn = db_arc.lock().unwrap();
                                if db::artwork_exists(&conn, &object_id).unwrap_or(false) {
                                    skipped += 1;
                                    continue;
                                }
                            }
                            let conn = db_arc.lock().unwrap();
                            match db::insert_artwork(
                                &conn, &object_id, &photo.title(), &photo.user.name,
                                &photo.year(), &photo.urls.regular, &photo.links.html,
                                "Unsplash", query, None,
                            ) {
                                Ok(_)  => inserted += 1,
                                Err(e) => println!("  Unsplash DB error: {e}"),
                            }
                        }
                        println!(
                            "Unsplash [{query}]: {} fetched | {inserted} inserted | {skipped} skipped | quota left: {remaining}",
                            photos.len()
                        );
                        // Extra sleep so Met + AIC time + this sleep >= safe inter-call gap
                        if unsplash_queries.get(i + 1).is_some() && remaining >= 3 {
                            sleep(Duration::from_secs(crate::UNSPLASH_INTER_QUERY_SLEEP)).await;
                        }
                    }
                    Err(e) => println!("Unsplash error [{query}]: {e}"),
                }
            }
        }
    }

    println!(">>> ingestion complete");
}
