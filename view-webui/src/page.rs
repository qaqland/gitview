use askama::Template;

use crate::data::{RepositoryPort, RepositoryRecord, ResolvedRev, short_hash};
use crate::debug_timing;
use crate::error::AppError;
pub use crate::url::path_route;
use crate::url::{display_rev, query, repo_route, valid_path};
use crate::web::AppState;

pub mod blob;
pub mod commit;
pub mod diff;
pub mod history;
pub mod refs;
pub mod repo_list;
pub mod tree;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PathView {
    Tree,
    Blob,
    Log,
}

impl PathView {
    pub fn op(self) -> &'static str {
        match self {
            Self::Tree => "tree",
            Self::Blob => "blob",
            Self::Log => "log",
        }
    }
}

#[derive(Clone)]
pub struct HeaderStatus {
    pub kind: &'static str,
    pub value: String,
    pub refs_href: Option<String>,
}

#[derive(Clone)]
pub struct Crumb {
    pub label: String,
    pub href: String,
    pub current: bool,
    pub mid: bool,
}

#[derive(Clone)]
pub struct ActionLink {
    pub label: &'static str,
    pub href: String,
}

#[derive(Clone)]
pub struct PathRow {
    pub description: String,
    pub root_repo_href: Option<String>,
    pub crumbs: Vec<Crumb>,
    pub actions: Vec<ActionLink>,
    pub current_action: &'static str,
    pub ellipsis: bool,
}

#[derive(Clone)]
pub struct RepoHeader {
    pub name: String,
    pub home_href: String,
    pub status: Option<HeaderStatus>,
    pub path: Option<PathRow>,
}

impl RepoHeader {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            home_href: format!("{}/-/tree", repo_route(name)),
            status: None,
            path: None,
        }
    }
}

pub struct RepoContext {
    pub repo: RepositoryRecord,
    pub default_rev: String,
    pub rev: ResolvedRev,
}

#[derive(Template)]
#[template(path = "header.html")]
struct HeaderTemplate<'a> {
    header: Option<&'a RepoHeader>,
}

#[derive(Template)]
#[template(path = "page.html")]
struct PageTemplate {
    title: String,
    header_html: String,
    alternate_markdown_href: String,
    alternate_text_href: String,
    content_html: String,
    generated: String,
    version: &'static str,
}

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorTemplate {
    status: u16,
    reason: String,
    message: String,
    location: String,
    root_href: String,
    refs_href: String,
    back_href: String,
}

pub fn render(
    title: &str,
    header: Option<&RepoHeader>,
    alternate: Option<(&str, &str)>,
    content_html: String,
) -> Result<String, AppError> {
    let header_html = {
        let _timer = debug_timing::start("render.header");
        HeaderTemplate { header }
            .render()
            .map_err(AppError::internal)?
    };
    let (alternate_markdown_href, alternate_text_href) = match alternate {
        Some((md, txt)) => (md.to_string(), txt.to_string()),
        None => (String::new(), String::new()),
    };
    let page = PageTemplate {
        title: title.to_string(),
        header_html,
        alternate_markdown_href,
        alternate_text_href,
        content_html,
        generated: debug_timing::request_elapsed()
            .map(|elapsed| format!("{:.3}", elapsed.as_secs_f64()))
            .unwrap_or_default(),
        version: env!("CARGO_PKG_VERSION"),
    };
    let _timer = debug_timing::start("render.layout");
    page.render().map_err(AppError::internal)
}

pub fn render_error(error: &AppError) -> String {
    let header = error.repo.as_ref().map(|repo| RepoHeader::new(&repo.name));
    let body = {
        let _timer = debug_timing::start("render.error_body");
        ErrorTemplate {
            status: error.status.as_u16(),
            reason: error
                .status
                .canonical_reason()
                .unwrap_or("Error")
                .to_string(),
            message: error.message.clone(),
            location: error
                .rev
                .as_ref()
                .map(|rev| {
                    format!(
                        "{} at {}",
                        error
                            .path
                            .as_deref()
                            .filter(|p| !p.is_empty())
                            .unwrap_or("Repository root"),
                        display_rev(rev)
                    )
                })
                .unwrap_or_default(),
            root_href: header
                .as_ref()
                .map(|h| path_route(&h.name, "tree", error.rev.as_deref().unwrap_or(""), ""))
                .unwrap_or_else(|| "/".to_string()),
            refs_href: header
                .as_ref()
                .map(|h| format!("{}/-/refs", repo_route(&h.name)))
                .unwrap_or_default(),
            back_href: error.back_href.clone(),
        }
        .render()
        .unwrap_or_else(|_| "template error".to_string())
    };
    render("Error", header.as_ref(), None, body).unwrap_or_else(|_| {
        format!(
            "<!doctype html><html><body><h1>Error {}</h1></body></html>",
            error.status.as_u16()
        )
    })
}

pub async fn repo_context(state: &AppState, repo_name: &str) -> Result<RepoContext, AppError> {
    let repo = require_repo(state, repo_name).await?;
    let default_rev = state
        .repo_port
        .default_rev(&repo)
        .await
        .map_err(|e| AppError::from_domain(e, Some(repo.clone())))?;
    let rev = state
        .repo_port
        .resolve_rev(repo.id, &default_rev)
        .await
        .map_err(|e| AppError::from_domain(e, Some(repo.clone())))?;
    Ok(RepoContext {
        repo,
        default_rev,
        rev,
    })
}

pub async fn repo_context_at_rev(
    state: &AppState,
    repo_name: &str,
    rev_name: &str,
) -> Result<RepoContext, AppError> {
    let repo = require_repo(state, repo_name).await?;
    repo_context_at_rev_for_repo(state, repo, rev_name).await
}

pub async fn repo_context_at_rev_for_repo(
    state: &AppState,
    repo: RepositoryRecord,
    rev_name: &str,
) -> Result<RepoContext, AppError> {
    let default_rev = state
        .repo_port
        .default_rev(&repo)
        .await
        .map_err(|e| AppError::from_domain(e, Some(repo.clone())))?;
    let rev = state
        .repo_port
        .resolve_rev(repo.id, rev_name)
        .await
        .map_err(|e| AppError::from_domain(e, Some(repo.clone())))?;
    Ok(RepoContext {
        repo,
        default_rev,
        rev,
    })
}

pub async fn require_repo(state: &AppState, repo_name: &str) -> Result<RepositoryRecord, AppError> {
    state
        .repo_port
        .get_repository(repo_name)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::not_found("repository not found", None))
}

pub fn split_tail(tail: &str, require_path: bool) -> Result<(String, String), AppError> {
    let parts: Vec<&str> = tail.split('/').filter(|part| !part.is_empty()).collect();
    if parts.is_empty() {
        return Err(AppError::not_found("revision not found", None));
    }

    let (rev, path_start) = if parts[0] == "tag" {
        if parts.len() < 2 {
            return Err(AppError::not_found("tag revision not found", None));
        }
        (format!("tag/{}", parts[1]), 2)
    } else {
        (parts[0].to_string(), 1)
    };
    let path = parts[path_start..].join("/");
    if !valid_path(&path) {
        return Err(AppError::not_found("invalid repository path", None));
    }
    if require_path && path.is_empty() {
        return Err(AppError::not_found("path not found", None));
    }
    Ok((rev, path))
}

pub fn join_path(parent: &str, child: &str) -> String {
    if parent.is_empty() {
        child.to_string()
    } else {
        format!("{parent}/{child}")
    }
}

pub fn parent_path(path: &str) -> String {
    path.rsplit_once('/')
        .map(|(parent, _)| parent.to_string())
        .unwrap_or_default()
}

pub fn refs_base(repo: &str, mode: &str, path: &str) -> String {
    let encoded = path
        .split('/')
        .filter(|part| !part.is_empty())
        .map(crate::url::segment)
        .collect::<Vec<_>>()
        .join("/");
    if encoded.is_empty() {
        format!("{}/-/refs/{mode}", repo_route(repo))
    } else {
        format!("{}/-/refs/{mode}/{encoded}", repo_route(repo))
    }
}

pub fn header_status(repo: &str, rev: &ResolvedRev, path: &str, view: PathView) -> HeaderStatus {
    let is_commit = rev.input.len() >= 8 && rev.input.bytes().all(|b| b.is_ascii_hexdigit());
    HeaderStatus {
        kind: view.op(),
        value: if is_commit {
            short_hash(&rev.commit_hash)
        } else {
            display_rev(&rev.input)
        },
        refs_href: Some(refs_base(repo, view.op(), path)),
    }
}

pub fn commit_status(commit_hash: &str) -> HeaderStatus {
    HeaderStatus {
        kind: "commit",
        value: short_hash(commit_hash),
        refs_href: None,
    }
}

pub fn path_row(repository: &RepositoryRecord, rev: &str, path: &str, is_file: bool, view: PathView) -> PathRow {
    let repo = &repository.name;
    let mut crumbs = if path.is_empty() {
        Vec::new()
    } else {
        vec![Crumb {
            label: repo.to_string(),
            href: path_route(repo, "tree", rev, ""),
            current: false,
            mid: false,
        }]
    };
    let parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    let mut current = String::new();
    for (idx, part) in parts.iter().enumerate() {
        if !current.is_empty() {
            current.push('/');
        }
        current.push_str(part);
        let last = idx + 1 == parts.len();
        let op = match (view, last, is_file) {
            (PathView::Log, _, _) => "log",
            (PathView::Blob, true, _) => "blob",
            _ => "tree",
        };
        crumbs.push(Crumb {
            label: part.to_string(),
            href: path_route(repo, op, rev, &current),
            current: last,
            mid: false,
        });
    }
    if crumbs.len() > 2 {
        let cut = crumbs.len() - 2;
        for crumb in crumbs.iter_mut().take(cut) {
            crumb.mid = true;
        }
    }

    let action = |label, href| ActionLink { label, href };
    let actions = match view {
        PathView::Tree => vec![action("History", path_route(repo, "log", rev, path))],
        PathView::Blob => {
            let mut actions = vec![
                action("History", path_route(repo, "log", rev, path)),
                action("Raw", path_route(repo, "raw", rev, path)),
            ];
            if !crate::data::git::is_preview_path(path) {
                actions.push(action("Diff", path_route(repo, "diff", rev, path)));
            }
            actions.push(action("Blob", query(&path_route(repo, "blob", rev, path), &[("view", "code")])));
            actions
        },
        PathView::Log => {
            let tree_path = if is_file {
                parent_path(path)
            } else {
                path.to_string()
            };
            let mut actions = vec![
                action("Tree", path_route(repo, "tree", rev, &tree_path)),
                action("History", path_route(repo, "log", rev, path)),
            ];
            if is_file {
                actions.push(action("Raw", path_route(repo, "raw", rev, path)));
            }
            actions
        }
    };
    PathRow {
        description: if path.is_empty() {
            repo_list::read_description(&repository.path)
        } else {
            String::new()
        },
        root_repo_href: path.is_empty().then(|| repo_route(repo)),
        ellipsis: crumbs.len() > 2,
        crumbs,
        actions,
        current_action: if view == PathView::Log { "History" } else { "" },
    }
}
