use reqwest::Client;
use serde_json::{json, Value};
use std::env;

pub struct Verdict {
    pub verdict: String,       // "approved", "rejected", "skipped"
    pub reasoning: String,
    pub confidence: f64,       // 0.0 - 1.0
    pub tags: Vec<String>,     // e.g. ["floral", "soft light", "reclining figure"]
}

pub async fn analyze_artwork(
    title: &str,
    artist: &str,
    date: &str,
    image_url: &str,
    examples: &[(String, String, String, String, String)], // from get_review_examples()
) -> anyhow::Result<Verdict> {
    let api_key = env::var("ANTHROPIC_API_KEY")?;
    let client = Client::new();

    // Build few-shot examples block
    let examples_text = if examples.is_empty() {
        "No examples yet — use your best judgment.".to_string()
    } else {
        examples.iter().map(|(title, artist, _, verdict, reasoning)| {
            format!("- \"{title}\" by {artist} → {verdict}: {reasoning}")
        }).collect::<Vec<_>>().join("\n")
    };

    let prompt = format!(r#"
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
        "#
    );

    let body = json!({
        "model": "claude-opus-4-6",
        "max_tokens": 300,
        "messages": [{
            "role": "user",
            "content": prompt
        }]
    });

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await?;

    let json: Value = response.json().await?;
    let text = json["content"][0]["text"].as_str().unwrap_or("{}");
    let parsed: Value = serde_json::from_str(text)?;

    Ok(Verdict {
        verdict: parsed["verdict"].as_str().unwrap_or("skipped").to_string(),
        reasoning: parsed["reasoning"].as_str().unwrap_or("").to_string(),
        confidence: parsed["confidence"].as_f64().unwrap_or(0.5),
        tags: parsed["tags"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|t| t.as_str().map(|s| s.to_string()))
            .collect(),
    })
}