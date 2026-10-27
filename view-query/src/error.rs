use thiserror::Error;

#[derive(Debug, Error)]
pub enum QueryError {
    #[error("repository not found")]
    RepoNotFound,
    #[error("repository HEAD not set")]
    HeadNotSet,
    #[error("revision not found")]
    RevNotFound,
    #[error("ambiguous commit hash")]
    AmbiguousHash,
    #[error("commit hash must contain at least 8 hexadecimal digits")]
    InvalidHash,
    #[error("limit is too large")]
    InvalidLimit,
    #[error("cursor is not on the requested history chain")]
    InvalidCursor,
    #[error("index is incomplete or inconsistent: {0}")]
    IncompleteIndex(&'static str),
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}
