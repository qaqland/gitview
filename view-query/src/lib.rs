//! Synchronous, read-only queries over a view-index SQLite database.
//! Connections, async scheduling, result caches and presentation belong to callers.

mod error;
mod history;
mod repository;

pub use error::QueryError;
pub use history::{HistoryPage, latest_change, log, path_history};
pub use repository::{
    RefKind, RefRecord, RepositoryRecord, ResolvedRev, get_repository, head_rev, list_refs,
    list_repositories, ref_time, resolve_hash, resolve_ref, resolve_rev,
};
pub use rusqlite::Connection;

use std::{path::Path, time::Duration};

pub type Result<T> = std::result::Result<T, QueryError>;

/// Apply once per connection, including connections supplied by an external pool.
pub fn configure_connection(conn: &Connection) -> Result<()> {
    conn.set_prepared_statement_cache_capacity(64);
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.pragma_update(None, "query_only", true)?;
    conn.pragma_update(None, "foreign_keys", true)?;
    Ok(())
}

/// Never creates a missing database or opens it for writing.
pub fn open_read_only(path: impl AsRef<Path>) -> Result<Connection> {
    let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    configure_connection(&conn)?;
    Ok(conn)
}
