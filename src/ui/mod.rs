use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use duckdb::Connection;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use crate::claude::{analyze_artwork, Model};
use crate::db;

pub type Db = Arc<Mutex<Connection>>;

#[derive(Serialize)]
struct ArtworkResponse {
    object_id: String,
    title: String,
    artist: String,
    date: String,
    image_url: String,
    source_url: String,
}

#[derive(Serialize)]
struct ClaudeResponse {
    verdict: String,
    reasoning: String,
    confidence: f64,
    tags: Vec<String>,
}

#[derive(Deserialize)]
struct VerdictRequest {
    object_id: String,
    verdict: String, // "approved", "rejected", "skipped"
}

#[derive(Deserialize)]
struct AnalyzeRequest {
    object_id: String,
    title: String,
    artist: String,
    date: String,
    image_url: String,
}

pub async fn serve(db: Db) {
  let app = Router::new()
      .route("/", get(index))
      .route("/api/next", get(next_artwork))
      .route("/api/analyze", post(analyze))
      .route("/api/verdict", post(submit_verdict))
      .route("/api/stats", get(stats))
      .with_state(db);  // pass directly, no re-wrapping

  let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
  println!("→ UI running at http://localhost:3000");
  axum::serve(listener, app).await.unwrap();
}

async fn index() -> Html<&'static str> {
    Html(HTML)
}

async fn next_artwork(State(db): State<Db>) -> impl IntoResponse {
  let conn = db.lock().unwrap();
  match db::next_pending(&conn) {
      Ok(Some((object_id, title, artist, date, image_url, source_url))) => {
          Json(Some(ArtworkResponse { object_id, title, artist, date, image_url, source_url })).into_response()
      }
      Ok(None) => Json(Option::<ArtworkResponse>::None).into_response(),
      Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
  }
}
async fn analyze(
    State(db): State<Db>,
    Json(req): Json<AnalyzeRequest>,
) -> impl IntoResponse {
    let examples = {
        let conn = db.lock().unwrap();
        db::get_review_examples(&conn, 20).unwrap_or_default()
    };

    match analyze_artwork(&req.title, &req.artist, &req.date, &req.image_url, &examples, Model::ClaudeOpus).await {
        Ok(verdict) => {
            // Save Claude's verdict to db
            let conn = db.lock().unwrap();
            let _ = db::save_claude_verdict(&conn, &req.object_id, &verdict.verdict, &verdict.reasoning);
            Json(ClaudeResponse {
                verdict: verdict.verdict,
                reasoning: verdict.reasoning,
                confidence: verdict.confidence,
                tags: verdict.tags,
            }).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn submit_verdict(
    State(db): State<Db>,
    Json(req): Json<VerdictRequest>,
) -> impl IntoResponse {
    let conn = db.lock().unwrap();
    match db::save_user_verdict(&conn, &req.object_id, &req.verdict) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn stats(State(db): State<Db>) -> impl IntoResponse {
    let conn = db.lock().unwrap();
    match db::count_by_status(&conn) {
        Ok((pending, approved, rejected, skipped, posted)) => {
            Json(serde_json::json!({
                "pending": pending,
                "approved": approved,
                "rejected": rejected,
                "skipped": skipped,
                "posted": posted,
            })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

const HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>venuscollect / curate</title>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; }

  :root {
    --bg: #0e0e0e;
    --fg: #e8e4dc;
    --dim: #555;
    --approve: #a8c5a0;
    --reject: #c5a0a0;
    --skip: #888;
    --claude: #b0a8c5;
  }

  body {
    background: var(--bg);
    color: var(--fg);
    font-family: 'Courier New', monospace;
    font-size: 13px;
    height: 100vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  header {
    padding: 12px 20px;
    border-bottom: 1px solid #1e1e1e;
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-shrink: 0;
  }

  header span { color: var(--dim); }
  header strong { color: var(--fg); letter-spacing: 0.1em; }

  #stats { font-size: 11px; color: var(--dim); }

  main {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr 320px;
    overflow: hidden;
  }

  #image-panel {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    overflow: hidden;
  }

  #image-panel img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    display: block;
  }

  #image-panel .empty {
    color: var(--dim);
    text-align: center;
    line-height: 2;
  }

  #sidebar {
    border-left: 1px solid #1e1e1e;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  #metadata {
    padding: 20px;
    border-bottom: 1px solid #1e1e1e;
    flex-shrink: 0;
  }

  #metadata h2 {
    font-size: 14px;
    font-weight: normal;
    margin-bottom: 6px;
    line-height: 1.4;
  }

  #metadata p {
    color: var(--dim);
    font-size: 11px;
    line-height: 1.8;
  }

  #metadata a {
    color: var(--dim);
    text-decoration: none;
  }

  #metadata a:hover { color: var(--fg); }

  #claude-panel {
    padding: 20px;
    border-bottom: 1px solid #1e1e1e;
    flex: 1;
    overflow-y: auto;
  }

  #claude-panel .label {
    font-size: 10px;
    letter-spacing: 0.15em;
    color: var(--dim);
    text-transform: uppercase;
    margin-bottom: 12px;
  }

  #claude-verdict {
    font-size: 12px;
    color: var(--claude);
    margin-bottom: 8px;
  }

  #claude-reasoning {
    color: var(--dim);
    font-size: 11px;
    line-height: 1.7;
    margin-bottom: 12px;
  }

  #claude-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .tag {
    font-size: 10px;
    padding: 2px 8px;
    border: 1px solid #2a2a2a;
    color: var(--dim);
    border-radius: 2px;
  }

  #confidence-bar {
    height: 2px;
    background: #1e1e1e;
    margin-bottom: 12px;
    border-radius: 1px;
    overflow: hidden;
  }

  #confidence-fill {
    height: 100%;
    background: var(--claude);
    transition: width 0.3s ease;
  }

  #analyze-btn {
    width: 100%;
    padding: 8px;
    background: none;
    border: 1px solid #2a2a2a;
    color: var(--dim);
    font-family: inherit;
    font-size: 11px;
    cursor: pointer;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    transition: all 0.15s;
  }

  #analyze-btn:hover { border-color: var(--claude); color: var(--claude); }
  #analyze-btn:disabled { opacity: 0.3; cursor: default; }

  #actions {
    padding: 16px 20px;
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 8px;
    flex-shrink: 0;
  }

  .action-btn {
    padding: 10px 4px;
    border: 1px solid #2a2a2a;
    background: none;
    font-family: inherit;
    font-size: 11px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    cursor: pointer;
    transition: all 0.15s;
  }

  .action-btn:disabled { opacity: 0.3; cursor: default; }

  #btn-approve { color: var(--approve); }
  #btn-approve:hover:not(:disabled) { background: var(--approve); color: var(--bg); border-color: var(--approve); }

  #btn-reject { color: var(--reject); }
  #btn-reject:hover:not(:disabled) { background: var(--reject); color: var(--bg); border-color: var(--reject); }

  #btn-skip { color: var(--skip); }
  #btn-skip:hover:not(:disabled) { background: #333; border-color: #444; }

  #shortcuts {
    padding: 8px 20px 12px;
    font-size: 10px;
    color: #333;
    text-align: center;
    flex-shrink: 0;
  }
</style>
</head>
<body>

<header>
  <strong>VENUSCOLLECT</strong> <span>/ curate</span>
  <div id="stats">loading...</div>
</header>

<main>
  <div id="image-panel">
    <div class="empty" id="empty-state" style="display:none">
      queue empty
    </div>
    <img id="artwork-img" src="" alt="" style="display:none">
  </div>

  <div id="sidebar">
    <div id="metadata">
      <h2 id="artwork-title">—</h2>
      <p id="artwork-artist">—</p>
      <p id="artwork-date">—</p>
      <p><a id="artwork-link" href="#" target="_blank">view source ↗</a></p>
    </div>

    <div id="claude-panel">
      <div class="label">Claude</div>
      <button id="analyze-btn" disabled>analyze</button>
      <div id="claude-output" style="display:none; margin-top: 14px;">
        <div id="confidence-bar"><div id="confidence-fill" style="width:0%"></div></div>
        <div id="claude-verdict"></div>
        <div id="claude-reasoning"></div>
        <div id="claude-tags"></div>
      </div>
    </div>

    <div id="actions">
      <button class="action-btn" id="btn-approve" disabled>approve</button>
      <button class="action-btn" id="btn-reject" disabled>reject</button>
      <button class="action-btn" id="btn-skip" disabled>skip</button>
    </div>

    <div id="shortcuts">a approve · r reject · s skip · c analyze</div>
  </div>
</main>

<script>
  let current = null;

  async function loadNext() {
    setButtons(false);
    document.getElementById('analyze-btn').disabled = true;
    document.getElementById('claude-output').style.display = 'none';
    document.getElementById('artwork-img').style.display = 'none';
    document.getElementById('empty-state').style.display = 'none';

    const res = await fetch('/api/next');
    const data = await res.json();

    if (!data) {
      document.getElementById('empty-state').style.display = 'block';
      document.getElementById('artwork-title').textContent = '—';
      document.getElementById('artwork-artist').textContent = '—';
      document.getElementById('artwork-date').textContent = '—';
      return;
    }

    current = data;
    const img = document.getElementById('artwork-img');
    img.onload = () => { img.style.display = 'block'; };
    img.src = data.image_url;

    document.getElementById('artwork-title').textContent = data.title;
    document.getElementById('artwork-artist').textContent = data.artist;
    document.getElementById('artwork-date').textContent = data.date;
    document.getElementById('artwork-link').href = data.source_url;
    document.getElementById('analyze-btn').disabled = false;
    setButtons(true);
    loadStats();
  }

  async function analyze() {
    if (!current) return;
    const btn = document.getElementById('analyze-btn');
    btn.disabled = true;
    btn.textContent = 'analyzing...';

    const res = await fetch('/api/analyze', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(current),
    });

    const data = await res.json();
    btn.textContent = 'done';

    document.getElementById('claude-output').style.display = 'block';
    document.getElementById('confidence-fill').style.width = (data.confidence * 100) + '%';
    document.getElementById('claude-verdict').textContent = data.verdict + ' (' + Math.round(data.confidence * 100) + '%)';
    document.getElementById('claude-reasoning').textContent = data.reasoning;

    const tagsEl = document.getElementById('claude-tags');
    tagsEl.innerHTML = data.tags.map(t => `<span class="tag">${t}</span>`).join('');
  }

  async function submitVerdict(verdict) {
    if (!current) return;
    setButtons(false);
    await fetch('/api/verdict', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ object_id: current.object_id, verdict }),
    });
    loadNext();
  }

  async function loadStats() {
    const res = await fetch('/api/stats');
    const s = await res.json();
    document.getElementById('stats').textContent =
      `${s.pending} pending · ${s.approved} approved · ${s.rejected} rejected · ${s.skipped} skipped`;
  }

  function setButtons(enabled) {
    ['btn-approve', 'btn-reject', 'btn-skip'].forEach(id => {
      document.getElementById(id).disabled = !enabled;
    });
  }

  document.getElementById('btn-approve').onclick = () => submitVerdict('approved');
  document.getElementById('btn-reject').onclick  = () => submitVerdict('rejected');
  document.getElementById('btn-skip').onclick    = () => submitVerdict('skipped');
  document.getElementById('analyze-btn').onclick = analyze;

  document.addEventListener('keydown', e => {
    if (e.key === 'a') submitVerdict('approved');
    if (e.key === 'r') submitVerdict('rejected');
    if (e.key === 's') submitVerdict('skipped');
    if (e.key === 'c') analyze();
  });

  loadNext();
  loadStats();
</script>
</body>
</html>
"##;