mod api;
mod oauth;
mod db;
mod media;
mod scheduler;

use oauth::OAuthConfig;
use twapi_v2::oauth10a::OAuthAuthentication;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect("Failed to load .env file");
    println!("venuscollect v0.1.0");

    // load creds
    let config = OAuthConfig::from_env();
    let oauth = OAuthAuthentication::new(
        config.consumer_key.clone(),
        config.consumer_secret.clone(),
        config.access_token.clone(),
        config.access_token_secret.clone(),
    );

    // test post
    let media_id = media::upload_image(&oauth, "src/assets/misskiahpregnancy_river.jpg").await.unwrap();
    api::post_tweet(&config, "Photography by: @misskiahphoto", Some(media_id))
        .await
        .unwrap();
}
