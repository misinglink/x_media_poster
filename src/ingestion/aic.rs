use reqwest::Client;
use serde::Deserialize;

const AIC_BASE: &str = "https://api.artic.edu/api/v1";
const AIC_IMAGE_BASE: &str = "https://www.artic.edu/iiif/2";

// ── Private response types ────────────────────────────────────────────────────

#[derive(Deserialize)]
struct AicSearchResponse {
    data: Vec<AicSearchItem>,
    pagination: AicPagination,
}

#[derive(Deserialize)]
struct AicPagination {
    total_pages: u32,
}

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AicSearchItem {
    pub id: u64,
    #[serde(rename = "_score")]
    pub score: Option<f64>,
}

#[derive(Deserialize, Debug)]
pub struct AicArtwork {
    pub id: u64,
    pub title: String,
    #[serde(rename = "artist_display")]
    pub artist: String,
    #[serde(rename = "date_display")]
    pub date: String,
    pub image_id: Option<String>,
    #[serde(rename = "is_public_domain")]
    pub is_public_domain: bool,
}

// ── API functions ─────────────────────────────────────────────────────────────

const AIC_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
    AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

/// Search AIC for public-domain paintings, drawings and prints.
pub async fn search_aic(query: &str) -> anyhow::Result<Vec<AicSearchItem>> {
    let client = Client::new();
    let mut all_items: Vec<AicSearchItem> = Vec::new();
    let mut page = 1u32;

    loop {
        let body = serde_json::json!({
            "q": query,
            "query": {
                "bool": {
                    "must": [
                        { "term": { "is_public_domain": true } },
                        { "terms": { "artwork_type_title.keyword": ["Painting", "Drawing and Watercolor", "Print"] } }
                    ]
                }
            },
            "fields": ["id"],
            "limit": 100,
            "page": page
        });

        let res: AicSearchResponse = client
            .post(format!("{}/artworks/search", AIC_BASE))
            .header("User-Agent", AIC_UA)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        let total_pages = res.pagination.total_pages;
        println!("  AIC [{query}] page {page}/{total_pages} ({} items)", res.data.len());
        all_items.extend(res.data);

        if page >= total_pages || page >= 10 {
            break;
        }
        page += 1;
    }

    Ok(all_items)
}

/// Fetch a single AIC artwork by ID.
/// Returns `None` if it isn't public domain or has no image.
pub async fn fetch_aic_artwork(id: u64) -> anyhow::Result<Option<AicArtwork>> {
    let client = Client::new();
    let url = format!(
        "{}/artworks/{}?fields=id,title,artist_display,date_display,image_id,is_public_domain",
        AIC_BASE, id
    );

    #[derive(Deserialize)]
    struct AicResponse { data: AicArtwork }

    let res: AicResponse = client
        .get(&url)
        .header("User-Agent", AIC_UA)
        .send()
        .await?
        .json()
        .await?;

    let art = res.data;
    if !art.is_public_domain || art.image_id.is_none() {
        return Ok(None);
    }

    Ok(Some(art))
}

/// Build an IIIF image URL for a given AIC image ID.
pub fn aic_image_url(image_id: &str) -> String {
    format!("{}/{}/full/full/0/default.jpg", AIC_IMAGE_BASE, image_id)
}
