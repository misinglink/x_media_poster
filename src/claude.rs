use reqwest::Client;
use serde_json::{json, Value};
use std::env;
use anyhow::Result;

pub enum Model {
    ClaudeOpus,
    ClaudeSonnet,
    ClaudeHaiku,
}

impl Model {
    pub fn as_str(&self) -> &'static str {
        match self {
            Model::ClaudeOpus   => "claude-opus-4-6",
            Model::ClaudeSonnet => "claude-sonnet-4-6",
            Model::ClaudeHaiku  => "claude-haiku-4-5-20251001",
        }
    }
}

/// Ask Claude (Haiku) to clean up raw museum metadata into tweet-ready text.
///
/// Rules encoded in the prompt:
/// - Title  : strip catalog codes, bracketed notes, trailing punctuation
/// - Artist : omit if "Unknown", "Anonymous", "After …", attributed noise, or empty
/// - Date   : omit if "Unknown", "N/A", "undated", empty, or doesn't read as a real period
/// - Output : title / artist (optional) / date (optional), one per line, ≤ 200 chars
///
/// For Unsplash photos call this with source = "Unsplash" and the function
/// returns an empty string immediately without hitting the API.
pub async fn compose_tweet_text(
    title:  &str,
    artist: &str,
    date:   &str,
    source: &str,
) -> Result<String> {
    // Unsplash photos get no caption — just the image
    if source == "Unsplash" {
        return Ok(String::new());
    }

    let api_key = env::var("ANTHROPIC_API_KEY")?;
    let client  = Client::new();

    let prompt = format!(r#"
You are writing tweet captions for @venuscollect, a classical art account.
Clean up these raw museum metadata fields into a minimal tweet caption.

Rules:
- Title  : clean it up — remove catalog codes, bracketed notes like [Painting], trailing periods, parenthetical references
- Artist : include ONLY if it is clearly a real person's name. Omit if the value is "Unknown", "Anonymous", starts with "After", "Attributed to", "Circle of", "Workshop of", "Follower of", or is otherwise uncertain/empty
- Date   : include ONLY if it reads as a meaningful period — e.g. "1847", "c. 1890", "17th century". Omit if it is empty, "Unknown", "N/A", "undated", or looks like a catalog number
- Format : title on line 1, artist on line 2 (if kept), date on line 3 (if kept). Nothing else — no hashtags, no emojis, no commentary
- Total  : ≤ 200 characters

Raw metadata:
Title:  {title}
Artist: {artist}
Date:   {date}

Reply with ONLY the tweet text — no explanation, no quotes around it.
"#);

    let body = json!({
        "model": Model::ClaudeHaiku.as_str(),
        "max_tokens": 120,
        "messages": [{ "role": "user", "content": prompt }]
    });

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key",          api_key)
        .header("anthropic-version",  "2023-06-01")
        .header("content-type",       "application/json")
        .json(&body)
        .send()
        .await?;

    let json: Value = response.json().await?;
    let text = json["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    Ok(text)
}

pub struct ClaudeVerdict {
    pub verdict:    String,
    pub reasoning:  String,
    pub confidence: f64,
    pub tags:       Vec<String>,
}

pub async fn analyze_artwork(
    title:     &str,
    artist:    &str,
    date:      &str,
    image_url: &str,
    examples:  &[(String, String, String, String, String)],
    model:     Model,
) -> Result<ClaudeVerdict> {
    let api_key = env::var("ANTHROPIC_API_KEY")?;
    let client  = Client::new();

    let examples_text = if examples.is_empty() {
        "No examples yet — use your best judgment.".to_string()
    } else {
        examples.iter().map(|(title, artist, _, verdict, reasoning)| {
            format!("- \"{title}\" by {artist} → {verdict}: {reasoning}")
        }).collect::<Vec<_>>().join("\n")
    };

    let prompt = format!(r##"
You are curating @venuscollect, an art bot with a specific aesthetic:
- Classical feminine figures, Venus/goddess mythology, Art Nouveau
- Soft, elegant, sensual but tasteful
- Nature, flowers, flowing forms
- High visual quality paintings and drawings

Past decisions:
{examples_text}

Now evaluate this artwork:
Title: {title}
Artist: {artist}
Date: {date}
Image: {image_url}

Respond ONLY with valid JSON, no markdown, no explanation outside the JSON:
{{
  "verdict": "approved" | "rejected" | "skipped",
  "reasoning": "one sentence explanation",
  "confidence": 0.0 to 1.0,
  "tags": ["tag1", "tag2", "tag3"]
}}

Tags should describe visual qualities NOT related to the search query (e.g. "soft light", "floral border", "reclining pose", "gold tones").
"##);

    let body = json!({
        "model": model.as_str(),
        "max_tokens": 300,
        "messages": [{ "role": "user", "content": prompt }]
    });

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await?;

    let json: Value  = response.json().await?;
    let text         = json["content"][0]["text"].as_str().unwrap_or("{}");
    let parsed: Value = serde_json::from_str(text)?;

    Ok(ClaudeVerdict {
        verdict:    parsed["verdict"].as_str().unwrap_or("skipped").to_string(),
        reasoning:  parsed["reasoning"].as_str().unwrap_or("").to_string(),
        confidence: parsed["confidence"].as_f64().unwrap_or(0.5),
        tags:       parsed["tags"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|t| t.as_str().map(|s| s.to_string()))
            .collect(),
    })
}