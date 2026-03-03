use duckdb::Connection;
use reqwest::Client;
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;

use crate::db;

const UNSPLASH_BASE: &str = "https://api.unsplash.com";
/// Unsplash API max per page
const PER_PAGE: u32 = 30;
/// Polite delay between pages within a single query
const INTER_PAGE_SLEEP: u64 = 3;
/// Stop the entire pass if remaining quota drops below this
const RATE_LIMIT_STOP_AT: u32 = 3;

// ── Public types ──────────────────────────────────────────────────────────────

/// A single photo returned by the Unsplash search endpoint.
/// Each search page returns full metadata — no per-photo detail call needed.
#[derive(Deserialize, Debug, Clone)]
pub struct UnsplashPhoto {
    /// Unique Unsplash ID (DB key: `unsplash_{id}`)
    pub id: String,
    pub description: Option<String>,
    pub alt_description: Option<String>,
    pub urls: UnsplashUrls,
    pub links: UnsplashLinks,
    pub user: UnsplashUser,
    /// ISO 8601 upload date, e.g. `"2021-04-15T10:30:00Z"`
    pub created_at: Option<String>,
}

impl UnsplashPhoto {
    /// Best-available title: alt_description → description → "Untitled"
    pub fn title(&self) -> String {
        self.alt_description
            .clone()
            .or_else(|| self.description.clone())
            .unwrap_or_else(|| "Untitled".to_string())
    }

    /// Year extracted from `created_at`, or "Unknown"
    pub fn year(&self) -> String {
        self.created_at
            .as_deref()
            .and_then(|s| s.get(..4))
            .unwrap_or("Unknown")
            .to_string()
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct UnsplashUrls {
    /// Regular size (~1080 px wide) — sufficient for tweet images
    pub regular: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct UnsplashLinks {
    /// Canonical Unsplash page for attribution
    pub html: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct UnsplashUser {
    pub name: String,
}

// ── Private response wrapper ──────────────────────────────────────────────────

#[derive(Deserialize)]
struct SearchResponse {
    total_pages: u32,
    results: Vec<UnsplashPhoto>,
}

// ── Ingestion orchestrator ────────────────────────────────────────────────────

/// Run a full Unsplash ingestion pass over every query in `queries`.
///
/// Rate-limit strategy (demo tier = 50 req/hour)
/// ──────────────────────────────────────────────
/// - `inter_query_sleep` seconds between queries prevents bursting through the
///   hourly quota (18 queries × 75 s ≈ 22 min per full pass).
/// - Reads `X-Ratelimit-Remaining` from each response and stops the entire
///   pass if fewer than `RATE_LIMIT_STOP_AT` (3) calls remain.
/// - Dedup check happens before any API call so repeat runs are cheap.
pub async fn run_unsplash_ingestion(
    db_arc: Arc<Mutex<Connection>>,
    queries: &[&str],
    access_key: &str,
    max_pages: u32,
    inter_query_sleep: u64,
) {
    println!(">>> [unsplash] ingestion started ({} queries)", queries.len());
    let mut global_remaining = 50u32;

    for (i, query) in queries.iter().enumerate() {
        if global_remaining < RATE_LIMIT_STOP_AT {
            println!(
                ">>> [unsplash] quota nearly exhausted ({global_remaining} left) — stopping pass"
            );
            break;
        }

        println!("\n  [unsplash] ({}/{}) {query}", i + 1, queries.len());

        match search_unsplash(query, access_key, max_pages).await {
            Ok((photos, remaining)) => {
                global_remaining = remaining;
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
                        &conn,
                        &object_id,
                        &photo.title(),
                        &photo.user.name,
                        &photo.year(),
                        &photo.urls.regular,
                        &photo.links.html,
                        "Unsplash",
                        query,
                        None,
                    ) {
                        Ok(_)  => inserted += 1,
                        Err(e) => println!("  [unsplash] DB insert error: {e}"),
                    }
                }

                println!(
                    "  [unsplash] {}: {} fetched | {inserted} inserted | {skipped} skipped | quota left: {remaining}",
                    query, photos.len()
                );
            }
            Err(e) => println!("  [unsplash] search error for '{query}': {e}"),
        }

        // Pace between queries — skip sleep after the last one
        if i + 1 < queries.len() && global_remaining >= RATE_LIMIT_STOP_AT {
            println!("  [unsplash] sleeping {inter_query_sleep}s…");
            sleep(Duration::from_secs(inter_query_sleep)).await;
        }
    }

    println!(">>> [unsplash] ingestion complete");
}

// ── Low-level search call ─────────────────────────────────────────────────────

/// Fetch up to `max_pages` pages of Unsplash portrait photos for `query`.
/// Returns `(photos, remaining_quota)`.
pub async fn search_unsplash(
    query: &str,
    access_key: &str,
    max_pages: u32,
) -> anyhow::Result<(Vec<UnsplashPhoto>, u32)> {
    let client = Client::new();
    let mut all_photos: Vec<UnsplashPhoto> = Vec::new();
    let mut page = 1u32;
    let mut remaining = 50u32;

    loop {
        let resp = client
            .get(format!("{}/search/photos", UNSPLASH_BASE))
            .query(&[
                ("query",       query),
                ("per_page",    &PER_PAGE.to_string()),
                ("page",        &page.to_string()),
                ("orientation", "portrait"),
            ])
            .header("Authorization",  format!("Client-ID {}", access_key))
            .header("Accept-Version", "v1")
            .send()
            .await?;

        // Read rate-limit header before consuming body
        if let Some(n) = resp
            .headers()
            .get("X-Ratelimit-Remaining")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u32>().ok())
        {
            remaining = n;
        }

        let body: SearchResponse = resp.json().await?;
        let total_pages = body.total_pages;
        all_photos.extend(body.results);

        let last_page = page >= total_pages || total_pages == 0;
        let hit_cap   = page >= max_pages;
        let low_quota = remaining < RATE_LIMIT_STOP_AT;

        if low_quota || last_page || hit_cap {
            break;
        }

        page += 1;
        sleep(Duration::from_secs(INTER_PAGE_SLEEP)).await;
    }

    Ok((all_photos, remaining))
}
