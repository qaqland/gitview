use rusqlite::{OptionalExtension, params};

use crate::{Connection, QueryError, Result, repository::commit_by_prefix};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HistoryPage {
    pub hashes: Vec<String>,
    /// Full hash of the last returned commit when another page exists.
    pub next_after: Option<String>,
}

/// Walk the first-parent chain, newest first. `after` is exclusive.
pub fn log(
    conn: &Connection,
    repo_id: i64,
    start_hash: &str,
    after: Option<&str>,
    limit: usize,
) -> Result<HistoryPage> {
    let fetch_limit = fetch_limit(limit)?;
    if limit == 0 {
        return Ok(HistoryPage::default());
    }
    let (input_id, _) = commit_by_prefix(conn, repo_id, start_hash)?;
    let start_id = match after {
        Some(cursor) => {
            let cursor_id = checked_cursor(conn, repo_id, input_id, cursor)?;
            first_parent(conn, cursor_id)?
        }
        None => Some(input_id),
    };
    let Some(start_id) = start_id else {
        return Ok(HistoryPage::default());
    };
    let mut stmt = conn.prepare_cached(
        "WITH RECURSIVE history(commit_id, seq) AS (
             SELECT ?1
                  , 0
             UNION ALL
             SELECT a.ancestor_id
                  , h.seq + 1
               FROM history AS h
               JOIN ancestors AS a
                 ON a.commit_id = h.commit_id
                AND a.exponent = 0
              ORDER BY 2
              LIMIT ?2
          )
          SELECT c.commit_hash
               , c.parent_hash
               , a.ancestor_id
            FROM history AS h
            JOIN commits AS c
              ON c.commit_id = h.commit_id
          LEFT JOIN ancestors AS a
              ON a.commit_id = c.commit_id
             AND a.exponent = 0
           ORDER BY h.seq;",
    )?;
    let mut rows = stmt.query(params![start_id, fetch_limit])?;
    let mut hashes = Vec::new();
    while let Some(row) = rows.next()? {
        let parent: Option<String> = row.get(1)?;
        let ancestor: Option<i64> = row.get(2)?;
        if parent.is_some() && ancestor.is_none() {
            return Err(QueryError::IncompleteIndex("missing first parent"));
        }
        hashes.push(row.get(0)?);
    }
    Ok(page(hashes, limit))
}

/// Walk indexed changes on the first-parent chain. Paths are exact: `dir/` is a
/// directory, `dir` is a file. Root history uses `log`, not an empty path.
pub fn path_history(
    conn: &Connection,
    repo_id: i64,
    path: &str,
    start_hash: &str,
    after: Option<&str>,
    limit: usize,
) -> Result<HistoryPage> {
    let fetch_limit = fetch_limit(limit)?;
    if limit == 0 {
        return Ok(HistoryPage::default());
    }
    let (input_id, _) = commit_by_prefix(conn, repo_id, start_hash)?;
    let mut stmt = conn.prepare_cached(
        "SELECT path_id
           FROM paths
          WHERE name = ?1;",
    )?;
    let path_id: Option<i64> = stmt.query_row([path], |row| row.get(0)).optional()?;
    let Some(path_id) = path_id else {
        return if after.is_some() {
            Err(QueryError::InvalidCursor)
        } else {
            Ok(HistoryPage::default())
        };
    };
    let start_id = match after {
        Some(cursor) => {
            let cursor_id = checked_cursor(conn, repo_id, input_id, cursor)?;
            let mut stmt = conn.prepare_cached(
                "SELECT last_commit_id
                   FROM changes
                  WHERE commit_id = ?1
                    AND path_id = ?2;",
            )?;
            let last: Option<i64> = stmt
                .query_row(params![cursor_id, path_id], |row| row.get(0))
                .optional()?
                .ok_or(QueryError::InvalidCursor)?;
            let last = last.ok_or(QueryError::IncompleteIndex("path chain not backfilled"))?;
            (last != cursor_id).then_some(last)
        }
        None => find_path_start(conn, repo_id, path_id, input_id)?,
    };
    let Some(start_id) = start_id else {
        return Ok(HistoryPage::default());
    };
    let mut stmt = conn.prepare_cached(
        "WITH RECURSIVE history(commit_id, seq) AS (
             SELECT ?1
                  , 0
             UNION ALL
             SELECT cg.last_commit_id
                  , h.seq + 1
               FROM history AS h
               JOIN changes AS cg
                 ON cg.commit_id = h.commit_id
                AND cg.path_id = ?2
              WHERE cg.last_commit_id != h.commit_id
              ORDER BY 2
              LIMIT ?3
          )
          SELECT c.commit_hash
               , cg.last_commit_id
            FROM history AS h
            JOIN commits AS c
              ON c.commit_id = h.commit_id
          LEFT JOIN changes AS cg
              ON cg.commit_id = h.commit_id
             AND cg.path_id = ?2
           ORDER BY h.seq;",
    )?;
    let mut rows = stmt.query(params![start_id, path_id, fetch_limit])?;
    let mut hashes = Vec::new();
    while let Some(row) = rows.next()? {
        let last: Option<i64> = row.get(1)?;
        if last.is_none() {
            return Err(QueryError::IncompleteIndex("path chain not backfilled"));
        }
        hashes.push(row.get(0)?);
    }
    Ok(page(hashes, limit))
}

pub fn latest_change(
    conn: &Connection,
    repo_id: i64,
    start_hash: &str,
    path: &str,
) -> Result<Option<String>> {
    Ok(path_history(conn, repo_id, path, start_hash, None, 1)?
        .hashes
        .into_iter()
        .next())
}

fn fetch_limit(limit: usize) -> Result<i64> {
    i64::try_from(limit)
        .ok()
        .and_then(|n| n.checked_add(1))
        .ok_or(QueryError::InvalidLimit)
}

fn page(mut hashes: Vec<String>, limit: usize) -> HistoryPage {
    let next_after = (hashes.len() > limit).then(|| hashes[limit - 1].clone());
    hashes.truncate(limit);
    HistoryPage { hashes, next_after }
}

fn first_parent(conn: &Connection, commit_id: i64) -> Result<Option<i64>> {
    let mut stmt = conn.prepare_cached(
        "SELECT c.parent_hash
              , a.ancestor_id
           FROM commits AS c
          LEFT JOIN ancestors AS a
             ON a.commit_id = c.commit_id
            AND a.exponent = 0
          WHERE c.commit_id = ?1;",
    )?;
    let (hash, id): (Option<String>, Option<i64>) =
        stmt.query_row([commit_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
    if hash.is_some() && id.is_none() {
        return Err(QueryError::IncompleteIndex("missing first parent"));
    }
    Ok(id)
}

fn depth(conn: &Connection, commit_id: i64) -> Result<i64> {
    let mut stmt = conn.prepare_cached(
        "SELECT first_depth
           FROM commits
          WHERE commit_id = ?1;",
    )?;
    let depth: Option<i64> = stmt.query_row([commit_id], |row| row.get(0))?;
    depth
        .filter(|n| *n >= 0)
        .ok_or(QueryError::IncompleteIndex("missing first-parent depth"))
}

fn is_first_parent_ancestor(conn: &Connection, candidate: i64, input: i64) -> Result<bool> {
    if candidate == input {
        return Ok(true);
    }
    let da = depth(conn, candidate)?;
    let db = depth(conn, input)?;
    if da > db {
        return Ok(false);
    }
    let mut current = input;
    let mut delta = db - da;
    let mut exponent = 0;
    let mut stmt = conn.prepare_cached(
        "SELECT ancestor_id
           FROM ancestors
          WHERE commit_id = ?1
            AND exponent = ?2;",
    )?;
    while delta > 0 {
        if delta & 1 == 1 {
            current = stmt
                .query_row(params![current, exponent], |row| row.get(0))
                .optional()?
                .ok_or(QueryError::IncompleteIndex("missing ancestor jump"))?;
        }
        delta >>= 1;
        exponent += 1;
    }
    Ok(current == candidate)
}

fn checked_cursor(conn: &Connection, repo_id: i64, input: i64, cursor: &str) -> Result<i64> {
    let (id, _) = commit_by_prefix(conn, repo_id, cursor)?;
    if !is_first_parent_ancestor(conn, id, input)? {
        return Err(QueryError::InvalidCursor);
    }
    Ok(id)
}

fn find_path_start(
    conn: &Connection,
    repo_id: i64,
    path_id: i64,
    input: i64,
) -> Result<Option<i64>> {
    let input_depth = depth(conn, input)?;
    let mut stmt = conn.prepare_cached(
        "SELECT cg.commit_id
           FROM changes AS cg
           JOIN commits AS c
             ON c.commit_id = cg.commit_id
          WHERE cg.path_id = ?1
            AND c.repository_id = ?2
            AND c.first_depth <= ?3
          ORDER BY c.first_depth DESC;",
    )?;
    let mut rows = stmt.query(params![path_id, repo_id, input_depth])?;
    while let Some(row) = rows.next()? {
        let candidate = row.get(0)?;
        if is_first_parent_ancestor(conn, candidate, input)? {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}
