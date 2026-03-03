use reqwest::Client;
use serde::Deserialize;

const MET_BASE: &str = "https://collectionapi.metmuseum.org/public/collection/v1";

#[derive(Deserialize, Debug)]
pub struct MetMuseumArtwork {
    #[serde(rename = "objectID")]
    pub object_id: u64,
    pub title: String,
    #[serde(rename = "artistDisplayName")]
    pub artist: String,
    #[serde(rename = "objectDate")]
    pub date: String,
    #[serde(rename = "primaryImageSmall")]
    pub image_url: String,
    #[serde(rename = "objectURL")]
    pub met_url: String,
    #[serde(rename = "isPublicDomain")]
    pub is_public_domain: bool,
}

/// Search the Met for public-domain artworks matching `query`.
/// Returns a list of object IDs.
pub async fn search_met(query: &str) -> anyhow::Result<Vec<u64>> {
    let client = Client::new();
    let url = format!(
        "{}/search?hasImages=true&isPublicDomain=true&q={}",
        MET_BASE, query
    );

    let val: serde_json::Value = client.get(&url).send().await?.json().await?;

    let ids = val["objectIDs"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_u64()).collect())
        .unwrap_or_default();

    Ok(ids)
}

/// Fetch full artwork details for a given Met object ID.
/// Returns `None` if the artwork has no image or isn't public domain.
pub async fn fetch_met_artwork(object_id: u64) -> anyhow::Result<Option<MetMuseumArtwork>> {
    let client = Client::new();
    let url = format!("{}/objects/{}", MET_BASE, object_id);
    let artwork: MetMuseumArtwork = client.get(&url).send().await?.json().await?;

    if artwork.image_url.is_empty() || !artwork.is_public_domain {
        return Ok(None);
    }

    Ok(Some(artwork))
}
