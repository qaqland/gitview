use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;

use super::{RepoHeader, refs_base, require_repo};
use crate::data::{EntryKind, GitPort, RefRecord, RepositoryPort, RepositoryRecord, short_hash};
use crate::error::{AppError, DomainError};
use crate::format::utc_time_without_zone;
use crate::url::{display_rev, path_route, query, valid_path};
use crate::web::AppState;

#[derive(Clone, Debug)]
pub struct RefsPage {
    pub repo: RepositoryRecord,
    pub branches: Vec<RefRecord>,
    pub tags: Vec<RefRecord>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SwitchView {
    Tree,
    Blob,
    Log,
    History,
}
impl SwitchView {
    fn op(self) -> &'static str {
        match self {
            Self::Tree => "tree",
            Self::Blob => "blob",
            Self::Log => "log",
            Self::History => "history",
        }
    }
}

#[derive(Clone, Copy)]
pub enum RefKind {
    Branches,
    Tags,
}

struct RefRow {
    name: String,
    href: String,
    commit_href: String,
    short: String,
    time: String,
    default: bool,
}

#[derive(Template)]
#[template(path = "refs.html")]
struct RefsTemplate {
    title: &'static str,
    show_branches: bool,
    show_tags: bool,
    branches: Vec<RefRow>,
    tags: Vec<RefRow>,
    switching: bool,
    context: String,
}

pub async fn handler(
    State(state): State<AppState>,
    Path(repo_name): Path<String>,
) -> Result<Response, AppError> {
    respond(state, repo_name, String::new(), None, None, None).await
}

pub async fn tags_handler(
    State(state): State<AppState>,
    Path(repo_name): Path<String>,
) -> Result<Response, AppError> {
    respond(
        state,
        repo_name,
        String::new(),
        None,
        None,
        Some(RefKind::Tags),
    )
    .await
}

pub async fn branches_handler(
    State(state): State<AppState>,
    Path(repo_name): Path<String>,
) -> Result<Response, AppError> {
    respond(
        state,
        repo_name,
        String::new(),
        None,
        None,
        Some(RefKind::Branches),
    )
    .await
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefsPathQuery {
    to: Option<String>,
}

pub async fn path_handler(
    State(state): State<AppState>,
    Path((repo_name, mode, path)): Path<(String, String, String)>,
    input: Result<Query<RefsPathQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Response, AppError> {
    path_respond(state, repo_name, mode, path, input).await
}

pub async fn path_root_handler(
    State(state): State<AppState>,
    Path((repo_name, mode)): Path<(String, String)>,
    input: Result<Query<RefsPathQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Response, AppError> {
    path_respond(state, repo_name, mode, String::new(), input).await
}

async fn path_respond(
    state: AppState,
    repo_name: String,
    mode: String,
    path: String,
    input: Result<Query<RefsPathQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Response, AppError> {
    let repo = require_repo(&state, &repo_name).await?;
    let Query(input) = input.map_err(|_| {
        AppError::bad_request("Invalid version switch parameters.", Some(repo.clone()))
    })?;
    let view = match mode.as_str() {
        "tree" => SwitchView::Tree,
        "blob" => SwitchView::Blob,
        "log" => SwitchView::Log,
        "history" => SwitchView::History,
        _ => {
            return Err(AppError::bad_request(
                "Unknown view in refs path.",
                Some(repo),
            ));
        }
    };
    respond(state, repo_name, path, Some(view), input.to, None).await
}

async fn respond(
    state: AppState,
    repo_name: String,
    path: String,
    view: Option<SwitchView>,
    to: Option<String>,
    kind: Option<RefKind>,
) -> Result<Response, AppError> {
    let repo = require_repo(&state, &repo_name).await?;
    if !valid_path(&path) {
        return Err(AppError::bad_request(
            "Invalid version switch context.",
            Some(repo),
        ));
    }
    if let (Some(target), Some(view)) = (&to, view) {
        let target_rev = state
            .repo_port
            .resolve_rev(repo.id, target)
            .await
            .map_err(|e| AppError::from_domain(e, Some(repo.clone())).with_path(target, &path))?;
        if !matches!(view, SwitchView::Log | SwitchView::History) {
            let kind = state
                .git_port
                .path_kind(repo.path.clone(), target_rev.commit_hash, path.clone())
                .await
                .map_err(|e| AppError::from_domain(e, Some(repo.clone())))?;
            let expected = match view {
                SwitchView::Tree => EntryKind::Tree,
                _ => EntryKind::Blob,
            };
            if kind != Some(expected) {
                return Err(AppError::not_found("path not found", Some(repo.clone()))
                    .with_path(target, &path));
            }
        }
        return Ok(
            Redirect::temporary(&path_route(&repo_name, view.op(), target, &path)).into_response(),
        );
    }
    let default = state
        .repo_port
        .default_rev(&repo)
        .await
        .map_err(|e| AppError::from_domain(e, Some(repo.clone())))?;
    let page = load(&state.repo_port, &repo_name)
        .await
        .map_err(|e| AppError::from_domain(e, Some(repo)))?;
    let title = match (view, kind) {
        (Some(_), _) => "Switch version",
        (None, Some(RefKind::Tags)) => "Tags",
        _ => "Branches",
    };
    let header = RepoHeader::new(&repo_name);
    let content = render_body(&page, view, &path, kind, &default, title)?;
    super::render(
        &format!("{repo_name} - {title}"),
        Some(&header),
        None,
        content,
    )
    .map(|html| Html(html).into_response())
}

pub async fn load<R: RepositoryPort>(
    repo_port: &R,
    repo_name: &str,
) -> Result<RefsPage, DomainError> {
    let repo = repo_port
        .get_repository(repo_name)
        .await?
        .ok_or(DomainError::RepoNotFound)?;
    let branches = repo_port.list_refs(repo.id, 0).await?;
    let tags = repo_port.list_refs(repo.id, 1).await?;
    Ok(RefsPage {
        repo,
        branches,
        tags,
    })
}

fn render_body(
    page: &RefsPage,
    view: Option<SwitchView>,
    path: &str,
    kind: Option<RefKind>,
    default: &str,
    title: &'static str,
) -> Result<String, AppError> {
    let _timer = crate::debug_timing::start("render.body");
    let repo = &page.repo.name;
    let rows = |refs: &[RefRecord], tags: bool| {
        refs.iter()
            .map(|reference| {
                let rev = if tags {
                    format!("tag/{}", reference.show_name)
                } else {
                    reference.show_name.clone()
                };
                let href = if let Some(view) = view {
                    query(&refs_base(repo, view.op(), path), &[("to", &rev)])
                } else {
                    path_route(repo, "tree", &rev, "")
                };
                RefRow {
                    name: display_rev(&rev),
                    href,
                    commit_href: path_route(repo, "commit", &reference.commit_hash, ""),
                    short: short_hash(&reference.commit_hash),
                    time: utc_time_without_zone(reference.time),
                    default: rev == default,
                }
            })
            .collect()
    };
    RefsTemplate {
        title,
        show_branches: !matches!(kind, Some(RefKind::Tags)),
        show_tags: !matches!(kind, Some(RefKind::Branches)),
        branches: rows(&page.branches, false),
        tags: rows(&page.tags, true),
        switching: view.is_some(),
        context: format!(
            "Continue viewing {} of {}.",
            match view {
                Some(SwitchView::Log) => "log",
                Some(SwitchView::History) => "history",
                _ => "content",
            },
            if path.is_empty() {
                "the repository root"
            } else {
                path
            }
        ),
    }
    .render()
    .map_err(AppError::internal)
}
