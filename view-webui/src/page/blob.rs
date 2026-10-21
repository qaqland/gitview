use askama::Template;
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::header,
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;

use super::{
    ActionLink, PathView, RepoHeader, header_status, path_route, path_row, repo_context_at_rev_for_repo,
    require_repo, split_tail,
};
use crate::data::git::{can_preview, is_markdown, is_preview_path, render_preview};
use crate::data::{BlobView, EntryKind, GitPort, RepositoryPort, RepositoryRecord, ResolvedRev};
use crate::error::{AppError, DomainError};
use crate::format::human_size;
use crate::web::AppState;

#[derive(Clone, Debug)]
pub struct BlobPage {
    pub repo: RepositoryRecord,
    pub rev: ResolvedRev,
    pub rev_name: String,
    pub path: String,
    pub blob: BlobView,
}

#[derive(Template)]
#[template(path = "blob.html")]
struct BlobTemplate {
    preview_unavailable: bool,
    preview_html: Option<String>,
    loc: usize,
    size: String,
    lines: Option<Vec<String>>,
    permalink: String,
    download_href: String,
}

#[derive(Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BlobViewMode {
    #[default]
    Preview,
    Code,
}

#[derive(Default, Deserialize)]
pub struct BlobQuery {
    #[serde(default)]
    view: BlobViewMode,
}

pub async fn handler(
    State(state): State<AppState>,
    Path((repo_name, tail)): Path<(String, String)>,
    input: Result<Query<BlobQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Response, AppError> {
    async {
        let repo = require_repo(&state, &repo_name).await?;
        let Query(input) =
            input.map_err(|_| AppError::bad_request("Invalid file view.", Some(repo.clone())))?;
        let (rev, path) =
            split_tail(&tail, false).map_err(|error| error.with_repo(repo.clone()))?;
        let ctx = repo_context_at_rev_for_repo(&state, repo, &rev)
            .await
            .map_err(|e| e.with_path(&rev, &path))?;
        let kind = state
            .git_port
            .path_kind(
                ctx.repo.path.clone(),
                ctx.rev.commit_hash.clone(),
                path.clone(),
            )
            .await
            .map_err(|e| AppError::from_domain(e, Some(ctx.repo.clone())).with_path(&rev, &path))?;
        if kind == Some(EntryKind::Tree) {
            return Ok(
                Redirect::temporary(&path_route(&repo_name, "tree", &rev, &path)).into_response(),
            );
        }
        if kind == Some(EntryKind::Commit) {
            return Err(AppError::not_found(
                "This path is a submodule, not a file in this repository.",
                Some(ctx.repo),
            )
            .with_path(&rev, &path));
        }
        let page = load(
            &state.repo_port,
            &state.git_port,
            &repo_name,
            &rev,
            path.clone(),
        )
        .await
        .map_err(|e| AppError::from_domain(e, Some(ctx.repo.clone())).with_path(&rev, &path))?;
        let previewable = can_preview(&page.blob.bytes, &page.path);
        let preview_html = if previewable && input.view == BlobViewMode::Preview {
            let _timer = crate::debug_timing::start("render.preview");
            let bytes = page.blob.bytes.clone();
            let repo = page.repo.name.clone();
            let rev = page.rev_name.clone();
            let path = page.path.clone();
            tokio::task::spawn_blocking(crate::debug_timing::bind_context(move || {
                render_preview(&bytes, &repo, &rev, &path)
            }))
                .await
                .map_err(AppError::internal)?
        } else {
            None
        };
        // Keep source anchors in preview responses: URL fragments are not sent to the server.
        let lines = render_lines(&page, state.config.highlight_size_limit).await;
        let current_action = if preview_html.is_some() { "Preview" } else { "Blob" };
        let content = render_body(&page, lines, previewable, preview_html)?;
        let raw_href = path_route(&page.repo.name, "raw", &page.rev_name, &page.path);
        let mut header = RepoHeader::new(&page.repo.name);
        header.status = Some(header_status(
            &page.repo.name,
            &page.rev,
            &page.path,
            PathView::Blob,
        ));
        let mut path = path_row(&page.repo, &page.rev_name, &page.path, true, PathView::Blob);
        if page.blob.text_lines.is_none() {
            path.actions.retain(|action| action.label != "Diff");
        }
        if previewable {
            path.actions.push(ActionLink {
                label: "Preview",
                href: path_route(&page.repo.name, "blob", &page.rev_name, &page.path),
            });
        }
        path.current_action = current_action;
        header.path = Some(path);
        let alternate = page.blob.text_lines.as_ref().map(|_| {
            (
                if is_markdown(&page.path) {
                    raw_href.as_str()
                } else {
                    ""
                },
                raw_href.as_str(),
            )
        });
        super::render(
            &format!("{} - {}", page.repo.name, page.path),
            Some(&header),
            alternate,
            content,
        )
        .map(|html| Html(html).into_response())
    }
    .await
}

#[derive(Default, Deserialize)]
pub struct RawQuery {
    download: Option<u8>,
}

pub async fn raw_blob(
    State(state): State<AppState>,
    Path((repo_name, tail)): Path<(String, String)>,
    query: Result<Query<RawQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Response, AppError> {
    async {
        let repo = require_repo(&state, &repo_name).await?;
        let Query(query) = query.map_err(|_| {
            AppError::bad_request("Invalid download parameters.", Some(repo.clone()))
        })?;
        let (rev, path) = split_tail(&tail, true).map_err(|error| error.with_repo(repo.clone()))?;
        let ctx = repo_context_at_rev_for_repo(&state, repo, &rev)
            .await
            .map_err(|e| e.with_path(&rev, &path))?;
        let blob = state
            .git_port
            .read_blob(ctx.repo.path.clone(), ctx.rev.commit_hash, path.clone())
            .await
            .map_err(|e| AppError::from_domain(e, Some(ctx.repo)).with_path(&rev, &path))?;
        let content_type = raw_content_type(&blob.bytes, blob.text_lines.is_some());
        let download = query.download == Some(1) || content_type == "application/octet-stream";
        let disposition = if download {
            format!(
                "attachment; filename*=UTF-8''{}",
                crate::url::segment(path.rsplit('/').next().unwrap_or("file")).replace(':', "%3A")
            )
        } else {
            "inline".to_string()
        };
        Ok((
            [
                (header::CONTENT_TYPE, content_type.to_string()),
                (header::CONTENT_DISPOSITION, disposition),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_string()),
                (
                    header::CONTENT_SECURITY_POLICY,
                    "default-src 'none'; sandbox".to_string(),
                ),
            ],
            Body::from(blob.bytes),
        )
            .into_response())
    }
    .await
}

fn raw_content_type(bytes: &[u8], text: bool) -> &'static str {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        "image/jpeg"
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        "image/gif"
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        "image/webp"
    } else if text {
        "text/plain; charset=utf-8"
    } else {
        "application/octet-stream"
    }
}

pub async fn load<R: RepositoryPort, G: GitPort>(
    repo_port: &R,
    git_port: &G,
    repo_name: &str,
    rev_name: &str,
    path: String,
) -> Result<BlobPage, DomainError> {
    let repo = repo_port
        .get_repository(repo_name)
        .await?
        .ok_or(DomainError::RepoNotFound)?;
    let rev = repo_port.resolve_rev(repo.id, rev_name).await?;
    let blob = git_port
        .read_blob(repo.path.clone(), rev.commit_hash.clone(), path.clone())
        .await?;
    Ok(BlobPage {
        repo,
        rev,
        rev_name: rev_name.to_string(),
        path,
        blob,
    })
}

/// Highlighted or HTML-escaped lines, ready for the template's `|safe`.
/// Highlighting is CPU-bound, so it runs on the blocking pool like the git calls.
async fn render_lines(page: &BlobPage, highlight_size_limit: i64) -> Option<Vec<String>> {
    let _timer = crate::debug_timing::start("render.source");
    let lines = page.blob.text_lines.clone()?;
    if !crate::highlight::within_size_limit(page.blob.size, highlight_size_limit) {
        return Some(escape_lines(&lines));
    }
    let path = page.path.clone();
    let highlighted = tokio::task::spawn_blocking(crate::debug_timing::bind_context(move || {
        crate::highlight::highlight_lines(&path, &lines).unwrap_or_else(|| escape_lines(&lines))
    }))
    .await
    .expect("highlight task panicked");
    Some(highlighted)
}

fn escape_lines(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .map(|line| html_escape::encode_text(line).into_owned())
        .collect()
}

fn render_body(
    page: &BlobPage,
    lines: Option<Vec<String>>,
    previewable: bool,
    preview_html: Option<String>,
) -> Result<String, AppError> {
    let _timer = crate::debug_timing::start("render.body");
    let raw_href = path_route(&page.repo.name, "raw", &page.rev.commit_hash, &page.path);
    let mut permalink = path_route(&page.repo.name, "blob", &page.rev.commit_hash, &page.path);
    if previewable {
        permalink = crate::url::query(&permalink, &[("view", "code")]);
    }
    BlobTemplate {
        preview_unavailable: is_preview_path(&page.path) && !previewable,
        preview_html,
        loc: page.blob.text_lines.as_ref().map_or(0, Vec::len),
        size: human_size(page.blob.size as u64),
        lines,
        permalink,
        download_href: crate::url::query(&raw_href, &[("download", "1")]),
    }
    .render()
    .map_err(AppError::internal)
}

#[cfg(test)]
mod tests {
    use super::raw_content_type;
    #[test]
    fn raw_never_executes_html_or_svg() {
        assert_eq!(
            raw_content_type(b"<script>alert(1)</script>", true),
            "text/plain; charset=utf-8"
        );
        assert_eq!(
            raw_content_type(b"<svg onload='alert(1)'></svg>", true),
            "text/plain; charset=utf-8"
        );
        assert_eq!(raw_content_type(b"\x89PNG\r\n\x1a\n", false), "image/png");
        assert_eq!(
            raw_content_type(b"\0binary", false),
            "application/octet-stream"
        );
    }
}
