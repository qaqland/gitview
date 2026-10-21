use std::path::PathBuf;

use clap::Parser;

/// Read-only web interface for browsing Git repositories indexed by view-index.
#[derive(Clone, Debug, Parser)]
#[command(version)]
pub struct AppConfig {
    /// SQLite database created by view-index
    #[arg(long, default_value = "view.db")]
    pub database: String,
    /// Address to bind the HTTP server to
    #[arg(long, default_value = "127.0.0.1:3000")]
    pub bind: String,
    /// Serve this CSS file instead of the built-in stylesheet (re-read per request)
    #[arg(long, value_name = "PATH")]
    pub css: Option<PathBuf>,
    /// Maximum file size for syntax highlighting in KiB (e.g. 256); 0 disables highlighting, negative always highlights
    #[arg(long, default_value = "256", value_parser = parse_size)]
    pub highlight_size_limit: i64,
}

fn parse_size(value: &str) -> Result<i64, String> {
    let digits = value.strip_suffix(['k', 'K']).unwrap_or(value);
    digits
        .parse::<i64>()
        .ok()
        .and_then(|size| size.checked_mul(1024))
        .ok_or_else(|| format!("invalid size {value:?}, expected KiB like 256"))
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            database: "test.db".to_string(),
            bind: "127.0.0.1:3000".to_string(),
            css: None,
            highlight_size_limit: 256 * 1024,
        }
    }
}
