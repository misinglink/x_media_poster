use std::fs;
use std::io::Cursor;
use twapi_v2::api::{
    post_2_media_upload_id_append, post_2_media_upload_id_finalize, post_2_media_upload_initialize,
};
use twapi_v2::api::post_2_media_upload_initialize::{MediaCategory};

use twapi_v2::oauth10a::OAuthAuthentication;

/// Uploads an image file to X and returns the media_id string.
/// X requires a 3-step process: INIT → APPEND → FINALIZE
pub async fn upload_image(
    auth: &OAuthAuthentication,
    image_path: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // read the image file
    let image_bytes = fs::read(image_path)?;
    let total_bytes = image_bytes.len();
    println!("Uploading image: {} ({} bytes)", image_path, total_bytes);

    // step 1: initialize the upload
    let init_body = post_2_media_upload_initialize::Body {
        media_type: "image/jpeg".to_string(),
        total_bytes: total_bytes as u64,
        media_category: Some(MediaCategory::TweetImage),
        ..Default::default()
    };

    let (init_res, _) = post_2_media_upload_initialize::Api::new(init_body)
        .execute(auth)
        .await?;
    let media_id = init_res.data.unwrap().id.unwrap().to_string();
    println!("Media ID: {}", media_id);

    // step 2: append the image data
    let append_body = post_2_media_upload_id_append::FormData {
        cursor: Cursor::new(image_bytes),
        segment_index: 0,
    };
    post_2_media_upload_id_append::Api::new(&media_id, append_body)
        .execute(auth)
        .await?;
    println!("Image uploaded successfully");

    // step 3: finalize the upload
    post_2_media_upload_id_finalize::Api::new(&media_id)
        .execute(auth)
        .await?;
    println!("Media finalized successfully");

    Ok(media_id)
}
