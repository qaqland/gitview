use askama::Template;
use axum::{
    extract::Path,
    extract::State,
    response::{Html, Redirect},
};

use crate::data::{
    EntryKind, GitPort, ReadmeView, RepositoryPort, RepositoryRecord, ResolvedRev, TreeEntryView,
};
use crate::error::{AppError, DomainError};
use crate::web::AppState;

use super::{
    PathView, RepoHeader, header_status, join_path, parent_path, path_route, path_row,
    repo_context_at_rev_for_repo, require_repo, split_tail,
};

#[derive(Clone, Debug)]
pub struct TreePage {
    pub repo: RepositoryRecord,
    pub rev: ResolvedRev,
    pub rev_name: String,
    pub path: String,
    pub entries: Vec<TreeEntryView>,
    pub readme: Option<ReadmeView>,
}

struct TreeRow {
    href: String,
    name: String,
    mode: String,
    size: String,
    kind_label: &'static str,
}

#[derive(Template)]
#[template(path = "tree.html")]
struct TreeTemplate<'a> {
    parent_href: Option<String>,
    rows: Vec<TreeRow>,
    readme: Option<&'a ReadmeView>,
    readme_href: String,
}

pub async fn default_handler(
    State(state): State<AppState>,
    Path(repo_name): Path<String>,
) -> Result<Redirect, AppError> {
    let ctx = super::repo_context(&state, &repo_name).await?;
    Ok(Redirect::temporary(&path_route(
        &repo_name,
        "tree",
        &ctx.default_rev,
        "",
    )))
}

pub async fn handler(
    State(state): State<AppState>,
    Path((repo_name, tail)): Path<(String, String)>,
) -> Result<Html<String>, AppError> {
    async {
        let repo = require_repo(&state, &repo_name).await?;
        let (rev, path) =
            split_tail(&tail, false).map_err(|error| error.with_repo(repo.clone()))?;
        let repo_ctx = repo_context_at_rev_for_repo(&state, repo, &rev)
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
        .map_err(|e| {
            AppError::from_domain(e, Some(repo_ctx.repo.clone())).with_path(&rev, &path)
        })?;
        let content = render_body(&page)?;
        let mut header = RepoHeader::new(&page.repo.name);
        header.status = Some(header_status(
            &page.repo.name,
            &page.rev,
            &page.path,
            PathView::Tree,
        ));
        header.path = Some(path_row(
            &page.repo,
            &page.rev_name,
            &page.path,
            false,
            PathView::Tree,
        ));
        let title = format!("{} - Tree {}", page.repo.name, page.path);
        super::render(&title, Some(&header), None, content).map(Html)
    }
    .await
}

pub async fn load<R: RepositoryPort, G: GitPort>(
    repo_port: &R,
    git_port: &G,
    repo_name: &str,
    rev_name: &str,
    path: String,
) -> Result<TreePage, DomainError> {
    let repo = repo_port
        .get_repository(repo_name)
        .await?
        .ok_or(DomainError::RepoNotFound)?;
    let rev = repo_port.resolve_rev(repo.id, rev_name).await?;
    let entries = git_port
        .list_tree(repo.path.clone(), rev.commit_hash.clone(), path.clone())
        .await?;
    let readme = git_port
        .read_readme(
            repo.path.clone(),
            rev.clone(),
            repo.name.clone(),
            path.clone(),
        )
        .await?;
    Ok(TreePage {
        repo,
        rev,
        rev_name: rev_name.to_string(),
        path,
        entries,
        readme,
    })
}

fn render_body(page: &TreePage) -> Result<String, AppError> {
    let _timer = crate::debug_timing::start("render.body");
    let repo_name = &page.repo.name;
    let rev = &page.rev_name;
    let path = &page.path;
    let parent_href =
        (!path.is_empty()).then(|| path_route(repo_name, "tree", rev, &parent_path(path)));
    let rows = page
        .entries
        .iter()
        .map(|entry| {
            let child_path = join_path(path, &entry.name);
            let op = match entry.kind {
                EntryKind::Blob => "blob",
                _ => "tree",
            };
            TreeRow {
                href: if matches!(entry.kind, EntryKind::Blob | EntryKind::Tree) {
                    path_route(repo_name, op, rev, &child_path)
                } else {
                    String::new()
                },
                name: entry.kind.display_name(&entry.name),
                mode: entry.mode.clone(),
                size: entry.kind.display_size(entry.size),
                kind_label: match entry.kind {
                    EntryKind::Commit => "Submodule",
                    EntryKind::Other => "Unsupported object",
                    EntryKind::Blob if entry.mode == "120000" => "Symbolic link",
                    EntryKind::Blob if entry.mode == "100755" => "Executable",
                    _ => "",
                },
            }
        })
        .collect();
    TreeTemplate {
        parent_href,
        rows,
        readme: page.readme.as_ref(),
        readme_href: page
            .readme
            .as_ref()
            .map(|readme| path_route(repo_name, "blob", rev, &readme.path))
            .unwrap_or_default(),
    }
    .render()
    .map_err(AppError::internal)
}
