use duckdb::{Connection, Result, params};

pub fn open() -> Result<Connection> {
    let conn = Connection::open("venuscollect.duckdb")?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<()> {
    // `source` column is VARCHAR (migration from the old source_type enum was
    // applied once and removed — the ALTER was causing WAL replay failures on
    // startup due to a DuckDB bug with DDL replay).
    // New databases get VARCHAR directly from the CREATE TABLE below.
    conn.execute_batch("
        CREATE TYPE IF NOT EXISTS status_type   AS ENUM ('pending', 'approved', 'rejected', 'skipped', 'posted');
        CREATE TYPE IF NOT EXISTS verdicts_type AS ENUM ('approved', 'rejected', 'pending', 'skipped');

        CREATE TABLE IF NOT EXISTS artworks (
            object_id           VARCHAR PRIMARY KEY UNIQUE NOT NULL,
            title               VARCHAR NOT NULL,
            artist              VARCHAR NOT NULL,
            date                VARCHAR NOT NULL,
            image_url           VARCHAR NOT NULL,
            source_url          VARCHAR NOT NULL,
            source              VARCHAR NOT NULL,
            query               VARCHAR,
            aic_score           FLOAT,

            status              status_type   NOT NULL DEFAULT 'pending',

            claude_verdict      verdicts_type NOT NULL DEFAULT 'pending',
            claude_reasoning    VARCHAR,
            claude_confidence   FLOAT,
            claude_tags         VARCHAR[],

            user_verdict        verdicts_type NOT NULL DEFAULT 'pending',

            ingested_at         TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            reviewed_at         TIMESTAMP,
            posted_at           TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS queries (
            id              INTEGER PRIMARY KEY,
            query           VARCHAR UNIQUE NOT NULL,
            last_fetched    TIMESTAMP
        );
    ")?;

    Ok(())
}

/// Returns true if the given object_id already exists in the artworks table.
/// Used to skip redundant API fetches during ingestion.
pub fn artwork_exists(conn: &Connection, object_id: &str) -> Result<bool> {
    let mut stmt = conn.prepare(
        "SELECT 1 FROM artworks WHERE object_id = ? LIMIT 1"
    )?;
    let mut rows = stmt.query(params![object_id])?;
    Ok(rows.next()?.is_some())
}

/// Insert an artwork, ignoring duplicates
pub fn insert_artwork(
    conn: &Connection,
    object_id: &str,
    title: &str,
    artist: &str,
    date: &str,
    image_url: &str,
    source_url: &str,
    source: &str,
    query: &str,
    aic_score: Option<f64>,
) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO artworks
            (object_id, title, artist, date, image_url, source_url, source, query, aic_score)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![object_id, title, artist, date, image_url, source_url, source, query, aic_score],
    )?;
    Ok(())
}

/// Get the next pending artwork for review
pub fn next_pending(conn: &Connection) -> Result<Option<(String, String, String, String, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT object_id, title, artist, date, image_url, source_url
         FROM artworks
         WHERE status = 'pending'
         ORDER BY random() LIMIT 1"
    )?;

    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        Ok(Some((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
        )))
    } else {
        Ok(None)
    }
}
/// Save Claude's verdict
pub fn save_claude_verdict(
    conn: &Connection,
    object_id: &str,
    verdict: &str,
    reasoning: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE artworks SET claude_verdict = ?, claude_reasoning = ? WHERE object_id = ?",
        params![verdict, reasoning, object_id],
    )?;
    Ok(())
}

/// Save your final verdict and update status
pub fn save_user_verdict(conn: &Connection, object_id: &str, verdict: &str) -> Result<()> {
    conn.execute(
        "UPDATE artworks
         SET user_verdict = ?, status = ?, reviewed_at = NOW()
         WHERE object_id = ?",
        params![verdict, verdict, object_id],
    )?;
    Ok(())
}

/// Get recent reviewed examples for Claude's few-shot learning
pub fn get_review_examples(conn: &Connection, limit: u32) -> Result<Vec<(String, String, String, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT title, artist, image_url, user_verdict, claude_reasoning
         FROM artworks
         WHERE user_verdict != 'pending'
         ORDER BY reviewed_at DESC
         LIMIT ?"
    )?;

    let rows = stmt.query_map(params![limit], |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
        ))
    })?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn count_by_status(conn: &Connection) -> Result<(u64, u64, u64, u64, u64)> {
    let mut stmt = conn.prepare(
        "SELECT status::VARCHAR, COUNT(*) FROM artworks GROUP BY status"
    )?;

    let mut pending  = 0u64;
    let mut approved = 0u64;
    let mut rejected = 0u64;
    let mut skipped  = 0u64;
    let mut posted   = 0u64;

    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
    })?;

    for row in rows.filter_map(|r| r.ok()) {
        match row.0.as_str() {
            "pending"  => pending  = row.1,
            "approved" => approved = row.1,
            "rejected" => rejected = row.1,
            "skipped"  => skipped  = row.1,
            "posted"   => posted   = row.1,
            _ => {}
        }
    }

    Ok((pending, approved, rejected, skipped, posted))
}

/// Get next approved unposted artwork.
/// Returns (object_id, title, artist, date, image_url, source).
pub fn next_unposted(conn: &Connection) -> Result<Option<(String, String, String, String, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT object_id, title, artist, date, image_url, source
         FROM artworks
         WHERE status = 'approved'
         ORDER BY random() LIMIT 1"
    )?;

    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        Ok(Some((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
        )))
    } else {
        Ok(None)
    }
}

/// Mark an artwork as posted
pub fn mark_posted(conn: &Connection, object_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE artworks SET status = 'posted', posted_at = NOW() WHERE object_id = ?",
        params![object_id],
    )?;
    Ok(())
}