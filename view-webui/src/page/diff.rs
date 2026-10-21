use askama::Template;
use axum::{
    extract::{Path, State},
    response::Html,
};

use super::{
    ActionLink, PathView, RepoHeader, header_status, path_route, path_row, repo_context_at_rev_for_repo, require_repo,
    split_tail,
};
use crate::data::{
    EntryKind, GitPort, PathDiffView, RepositoryPort, RepositoryRecord, ResolvedRev,
};
use crate::error::{AppError, DomainError};
use crate::web::AppState;

#[derive(Clone, Debug)]
pub struct DiffPage {
    pub repo: RepositoryRecord,
    pub rev: ResolvedRev,
    pub rev_name: String,
    pub path: String,
    pub diff: PathDiffView,
}

struct DiffLine {
    class: &'static str,
    text: String,
}

#[derive(Template)]
#[template(path = "diff.html")]
struct DiffTemplate {
    lines: Vec<DiffLine>,
}

pub async fn handler(
    State(state): State<AppState>,
    Path((repo_name, tail)): Path<(String, String)>,
) -> Result<Html<String>, AppError> {
    let repo = require_repo(&state, &repo_name).await?;
    let (rev, path) = split_tail(&tail, true).map_err(|error| error.with_repo(repo.clone()))?;
    let ctx = repo_context_at_rev_for_repo(&state, repo, &rev)
        .await
        .map_err(|e| e.with_path(&rev, &path))?;
    let page = load(
        &state.repo_port,
        &state.git_port,
        &repo_name,
        &rev,
        path.clone(),
    )
    .await
    .map_err(|e| AppError::from_domain(e, Some(ctx.repo.clone())).with_path(&rev, &path))?;
    let _timer = crate::debug_timing::start("render.body");
    let lines = page
        .diff
        .lines
        .iter()
        .map(|line| DiffLine {
            class: super::commit::diff_line_class(line),
            text: line.clone(),
        })
        .collect();
    let content = DiffTemplate { lines }
        .render()
        .map_err(AppError::internal)?;
    let mut header = RepoHeader::new(&page.repo.name);
    header.status = Some(header_status(
        &page.repo.name,
        &page.rev,
        &page.path,
        PathView::Blob,
    ));
    let mut path = path_row(&page.repo, &page.rev_name, &page.path, true, PathView::Blob);
    if !path.actions.iter().any(|action| action.label == "Diff") {
        path.actions.push(ActionLink {
            label: "Diff",
            href: path_route(&page.repo.name, "diff", &page.rev_name, &page.path),
        });
    }
    path.current_action = "Diff";
    header.path = Some(path);
    super::render(
        &format!("{} - Diff {}", page.repo.name, page.path),
        Some(&header),
        None,
        content,
    )
    .map(Html)
}

pub async fn load<R: RepositoryPort, G: GitPort>(
    repo_port: &R,
    git_port: &G,
    repo_name: &str,
    rev_name: &str,
    path: String,
) -> Result<DiffPage, DomainError> {
    let repo = repo_port
        .get_repository(repo_name)
        .await?
        .ok_or(DomainError::RepoNotFound)?;
    let rev = repo_port.resolve_rev(repo.id, rev_name).await?;
    let kind = git_port
        .path_kind(repo.path.clone(), rev.commit_hash.clone(), path.clone())
        .await?;
    if kind != Some(EntryKind::Blob) {
        return Err(DomainError::PathNotFound);
    }
    let diff = git_port
        .path_diff(repo.path.clone(), rev.commit_hash.clone(), path.clone())
        .await?;
    Ok(DiffPage {
        repo,
        rev,
        rev_name: rev_name.to_string(),
        path,
        diff,
    })
}
