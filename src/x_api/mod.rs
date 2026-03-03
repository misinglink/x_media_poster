use crate::oauth::OAuthConfig;
use twapi_v2::api::post_2_tweets;
use twapi_v2::oauth10a::OAuthAuthentication;

/// Posts a plain text tweet as @venuscollect
pub async fn post_tweet(
    config: &OAuthConfig,
    text: &str,
    media_id: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Build OAuth 1.0a auth using twapi-v2's built-in implementation
    let auth = OAuthAuthentication::new(
        config.consumer_key.clone(),
        config.consumer_secret.clone(),
        config.access_token.clone(),
        config.access_token_secret.clone(),
    );
    
    // if media_id is provided, add it to the tweet body
    let media = media_id.map(|id| post_2_tweets::Media {
        media_ids: vec![id],
        ..Default::default()
    });

    // Build the tweet body
    let body = post_2_tweets::Body {
        text: Some(text.to_string()),
        media,
        ..Default::default()
    };

    // Send the request
    let (res, _headers) = post_2_tweets::Api::new(body).execute(&auth).await?;

    // id is Option<String> so we use {:?} to print it
    println!("Tweet posted! ID: {:?}", res.data.unwrap().id);
    Ok(())
}
