use rusqlite::{OptionalExtension, params};

use crate::{Connection, QueryError, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositoryRecord {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub head: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i64)]
pub enum RefKind {
    Branch = 0,
    Tag = 1,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefRecord {
    pub full_name: String,
    pub show_name: String,
    pub commit_hash: String,
    pub time: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedRev {
    pub input: String,
    pub commit_hash: String,
}

pub fn list_repositories(conn: &Connection) -> Result<Vec<RepositoryRecord>> {
    let mut stmt = conn.prepare_cached(
        "SELECT repository_id
              , repository_name
              , repository_path
              , repository_head
           FROM repositories
          ORDER BY repository_name;",
    )?;
    let repositories = stmt
        .query_map([], read_repository)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(repositories)
}

pub fn get_repository(conn: &Connection, name: &str) -> Result<Option<RepositoryRecord>> {
    let mut stmt = conn.prepare_cached(
        "SELECT repository_id
              , repository_name
              , repository_path
              , repository_head
           FROM repositories
          WHERE repository_name = ?1;",
    )?;
    let repository = stmt.query_row([name], read_repository).optional()?;
    Ok(repository)
}

pub fn head_rev(conn: &Connection, repo: &RepositoryRecord) -> Result<ResolvedRev> {
    let head = repo.head.as_deref().ok_or(QueryError::HeadNotSet)?;
    let full_name = if head.starts_with("refs/") {
        head.to_owned()
    } else {
        format!("refs/heads/{head}")
    };
    resolve_ref(conn, repo.id, &full_name)
}

pub fn list_refs(conn: &Connection, repo_id: i64, kind: RefKind) -> Result<Vec<RefRecord>> {
    let mut stmt = conn.prepare_cached(
        "SELECT r.full_name
              , r.show_name
              , c.commit_hash
              , r.ref_time
           FROM refs AS r
           JOIN commits AS c
             ON c.commit_id = r.commit_id
            AND c.repository_id = r.repository_id
          WHERE r.repository_id = ?1
            AND r.ref_type = ?2
          ORDER BY r.ref_time DESC
                 , r.show_name;",
    )?;
    let refs = stmt
        .query_map(params![repo_id, kind as i64], |row| {
            let record = RefRecord {
                full_name: row.get(0)?,
                show_name: row.get(1)?,
                commit_hash: row.get(2)?,
                time: row.get(3)?,
            };
            Ok(record)
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(refs)
}

/// Timestamp of an exact full ref name, or the latest ref when no name is supplied.
pub fn ref_time(conn: &Connection, repo_id: i64, full_name: Option<&str>) -> Result<Option<i64>> {
    let mut stmt = conn.prepare_cached(
        "SELECT MAX(ref_time)
           FROM refs
          WHERE repository_id = ?1
            AND (?2 IS NULL OR full_name = ?2);",
    )?;
    let time = stmt.query_row(params![repo_id, full_name], |row| row.get(0))?;
    Ok(time)
}

pub fn resolve_ref(conn: &Connection, repo_id: i64, full_name: &str) -> Result<ResolvedRev> {
    let mut stmt = conn.prepare_cached(
        "SELECT c.commit_hash
           FROM refs AS r
           JOIN commits AS c
             ON c.commit_id = r.commit_id
            AND c.repository_id = r.repository_id
          WHERE r.repository_id = ?1
            AND r.full_name = ?2;",
    )?;
    let hash = stmt
        .query_row(params![repo_id, full_name], |row| row.get(0))
        .optional()?;
    let resolved = ResolvedRev {
        input: full_name.to_owned(),
        commit_hash: hash.ok_or(QueryError::RevNotFound)?,
    };
    Ok(resolved)
}

pub fn resolve_rev(conn: &Connection, repo_id: i64, rev: &str) -> Result<ResolvedRev> {
    if rev.starts_with("refs/") {
        return resolve_ref(conn, repo_id, rev);
    }
    for namespace in ["heads", "tags"] {
        match resolve_ref(conn, repo_id, &format!("refs/{namespace}/{rev}")) {
            Ok(mut resolved) => {
                resolved.input = rev.to_owned();
                return Ok(resolved);
            }
            Err(QueryError::RevNotFound) => {}
            Err(err) => return Err(err),
        }
    }
    if rev.len() >= 8 && rev.bytes().all(|b| b.is_ascii_hexdigit()) {
        resolve_hash(conn, repo_id, rev)
    } else {
        Err(QueryError::RevNotFound)
    }
}

pub fn resolve_hash(conn: &Connection, repo_id: i64, prefix: &str) -> Result<ResolvedRev> {
    let (_, commit_hash) = commit_by_prefix(conn, repo_id, prefix)?;
    let resolved = ResolvedRev {
        input: prefix.to_owned(),
        commit_hash,
    };
    Ok(resolved)
}

pub(crate) fn commit_by_prefix(
    conn: &Connection,
    repo_id: i64,
    prefix: &str,
) -> Result<(i64, String)> {
    if prefix.len() < 8 || !prefix.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(QueryError::InvalidHash);
    }
    let mut stmt = conn.prepare_cached(
        "SELECT commit_id
              , commit_hash
           FROM commits
          WHERE repository_id = ?1
            AND commit_hash LIKE ?2
          LIMIT 2;",
    )?;
    let mut rows = stmt.query(params![repo_id, format!("{prefix}%")])?;
    let row = rows.next()?.ok_or(QueryError::RevNotFound)?;
    let commit = (row.get(0)?, row.get(1)?);
    if rows.next()?.is_some() {
        return Err(QueryError::AmbiguousHash);
    }
    Ok(commit)
}

fn read_repository(row: &rusqlite::Row<'_>) -> rusqlite::Result<RepositoryRecord> {
    let head: Option<String> = row.get(3)?;
    let repository = RepositoryRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        path: row.get(2)?,
        head: head.map(|h| h.trim().to_owned()).filter(|h| !h.is_empty()),
    };
    Ok(repository)
}
