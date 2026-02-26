/// Holds X API credentials loaded from .env
pub struct OAuthConfig {
    pub consumer_key: String,
    pub consumer_secret: String,
    pub access_token: String,
    pub access_token_secret: String,
    // pub bearer_token: String,
}

impl OAuthConfig {
    pub fn from_env() -> Self {
        Self {
            consumer_key: std::env::var("X_CONSUMER_KEY").expect("X_CONSUMER_KEY not set"),
            consumer_secret: std::env::var("X_CONSUMER_SECRET").expect("X_CONSUMER_SECRET not set"),
            access_token: std::env::var("X_ACCESS_TOKEN").expect("X_ACCESS_TOKEN not set"),
            access_token_secret: std::env::var("X_ACCESS_TOKEN_SECRET")
                .expect("X_ACCESS_TOKEN_SECRET not set"),
            // bearer_token: std::env::var("X_BEARER_TOKEN").expect("X_BEARER_TOKEN not set"),
        }
    }
}
