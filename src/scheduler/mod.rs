use std::sync::{Arc, Mutex};
use std::time::Duration;

use duckdb::Connection;
use tokio::time::sleep;
use twapi_v2::oauth10a::OAuthAuthentication;

use crate::{claude, db, ingestion, media, oauth, x_api};

/// Runs indefinitely, posting one approved artwork to X every `interval_secs`.
pub async fn run_tweet_scheduler(db: Arc<Mutex<Connection>>, interval_secs: u64) {
    let config = oauth::OAuthConfig::from_env();
    let auth = OAuthAuthentication::new(
        config.consumer_key.clone(),
        config.consumer_secret.clone(),
        config.access_token.clone(),
        config.access_token_secret.clone(),
    );

    println!(">>> [scheduler] Tweet scheduler started — posting every {interval_secs} s");

    loop {
        println!(">>> [scheduler] tick — looking for next approved artwork…");

        // ── Pick artwork ────────────────────────────────────────────────────
        let artwork = {
            let conn = db.lock().unwrap();
            db::next_unposted(&conn).unwrap_or(None)
        };

        let (object_id, title, artist, date, image_url, source) = match artwork {
            None => {
                println!("  [scheduler] No approved artworks — skipping this tick");
                continue;
            }
            Some(row) => row,
        };

        println!("  [scheduler] Posting: \"{title}\" [{source}]");

        // ── Compose tweet text via Claude (Haiku) ───────────────────────────
        // • Unsplash → empty string (image-only post, no caption)
        // • Museum   → Claude strips catalog noise, drops uncertain artist/date
        let tweet_text = match claude::compose_tweet_text(&title, &artist, &date, &source).await {
            Ok(text) => text,
            Err(e) => {
                // Fall back gracefully rather than skipping the post
                println!("  [scheduler] Claude cleanup failed ({e}) — using raw fallback");
                if source == "Unsplash" {
                    String::new()
                } else {
                    format!("{title}\n{artist}\n{date}")
                }
            }
        };

        println!("  [scheduler] Caption: {:?}", tweet_text);

        // ── Download image ──────────────────────────────────────────────────
        let bytes = match ingestion::download_image(&image_url).await {
            Ok(b) => b,
            Err(e) => {
                println!("  [scheduler] Image download failed: {e}");
                continue;
            }
        };

        // ── Write to temp file ──────────────────────────────────────────────
        let safe_id  = object_id.replace(['/', '\\', ':'], "_");
        let tmp_path = format!("/tmp/venuscollect_{safe_id}.jpg");
        if let Err(e) = std::fs::write(&tmp_path, &bytes) {
            println!("  [scheduler] Failed to write temp image: {e}");
            continue;
        }

        // ── Upload media to X ───────────────────────────────────────────────
        let media_id = match media::upload_image(&auth, &tmp_path).await {
            Ok(id) => id,
            Err(e) => {
                println!("  [scheduler] Media upload failed: {e}");
                let _ = std::fs::remove_file(&tmp_path);
                continue;
            }
        };

        // ── Post tweet ──────────────────────────────────────────────────────
        // X accepts an empty-string body when a media_id is attached (image-only post)
        match x_api::post_tweet(&config, &tweet_text, Some(media_id)).await {
            Ok(_) => {
                let conn = db.lock().unwrap();
                let _ = db::mark_posted(&conn, &object_id);
                println!("  [scheduler] ✅ Posted: {object_id}");
            }
            Err(e) => println!("  [scheduler] Tweet post failed: {e}"),
        }

        let _ = std::fs::remove_file(&tmp_path);

        sleep(Duration::from_secs(interval_secs)).await;
    }
}
