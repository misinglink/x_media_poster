pub mod claude;
pub mod db;
pub mod ingestion;
pub mod media;
pub mod model_verdicts;
pub mod oauth;
pub mod scheduler;
pub mod ui;
pub mod x_api;

/// Art queries shared across all ingestion binaries.
pub const QUERIES: &[&str] = &[
    "jugendstil woman",
    "venus goddess", "diana goddess", "flora goddess",
    "persephone", "nymph", "feminine figure", "nude woman",
    "goddess nature", "mother nature", "art nouveau woman",
    "mucha", "klimt", "portrait flowers woman",
    "portrait garden woman", "woman flowers", "woman nature",
    "chinese flowers", "chinese botanical", "japanese flowers woman",
    "chinese painting flowers", "garden flowers painting",
    "botanical woman", "floral feminine",
    // romance / couples
    "couple embracing", "lovers embrace", "kiss painting",
    "romantic couple painting", "eternal love painting",
    "lovers mythology", "cupid psyche", "romeo juliet painting",
    "couple garden painting", "romantic kiss art",
];

/// Photography-tuned queries for Unsplash.
/// Kept separate from QUERIES because Unsplash is a photo platform —
/// museum-specific terms like "jugendstil" or "klimt" return near-zero results.
pub const UNSPLASH_QUERIES: &[&str] = &[
    "goddess portrait woman",
    "floral crown portrait",
    "woman flowers portrait",
    "nature goddess photography",
    "ethereal woman portrait",
    "romantic feminine portrait",
    "forest nymph photography",
    "botanical woman portrait",
    "classical beauty portrait",
    "art nouveau inspired portrait",
    "pre-raphaelite inspired photography",
    "woman garden portrait",
    "vintage feminine photography",
    "mythological woman portrait",
    "floral feminine portrait",
    "woman nature portrait",
    "dark romantic portrait",
    "renaissance inspired portrait",
    "woman flowers",
    "woman nature",
    "lingerie nature",
    "lingerie model",
    // romance / couples
    "couple embracing portrait",
    "lovers kiss portrait",
    "romantic couple photography",
    "eternal love portrait",
    "kissing couple nature",
    "lovers embrace photography",
    "romantic kiss portrait",
];

/// Unsplash pages to fetch per query (30 results/page).
/// 1 page × 26 queries = 26 requests — well within the 50 req/hour demo limit.
pub const UNSPLASH_MAX_PAGES: u32 = 1;

/// Seconds to sleep between Unsplash queries.
/// 18 queries × 75 s = ~22 min per full pass; keeps us under the hourly cap
/// even if the hour resets partway through.
pub const UNSPLASH_INTER_QUERY_SLEEP: u64 = 75;

/// Seconds between scheduled tweets (3600 = 1 hour).
pub const TWEET_INTERVAL_SECS: u64 = 3600;
