use askama::Template;
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Response},
};

use super::{ActionLink, PathRow, RepoHeader, commit_status, require_repo};
use crate::data::{
    CommitDiffView, CommitInfo, GitPort, RepositoryPort, RepositoryRecord, short_hash,
};
use crate::error::{AppError, DomainError};
use crate::url::path_route;
use crate::web::AppState;

#[derive(Clone, Debug)]
pub struct CommitPage {
    pub repo: RepositoryRecord,
    pub diff: CommitDiffView,
}

struct ChangedFileRow {
    path: String,
    old_path: Option<String>,
    status: String,
    additions: usize,
    deletions: usize,
    blob_href: String,
    view_label: &'static str,
    anchor: String,
    diff: Vec<DiffLine>,
    truncated: bool,
}

struct DiffLine {
    class: &'static str,
    text: String,
}
struct Parent {
    short: String,
    href: String,
}

#[derive(Template)]
#[template(path = "commit.html")]
struct CommitTemplate {
    commit: CommitInfo,
    parents: Vec<Parent>,
    message: String,
    files_changed: usize,
    insertions: usize,
    deletions: usize,
    files: Vec<ChangedFileRow>,
    patch_href: String,
    truncated: bool,
}

pub async fn handler(
    State(state): State<AppState>,
    Path((repo_name, hash)): Path<(String, String)>,
) -> Result<Response, AppError> {
    async {
        let patch = hash.ends_with(".patch");
        let hash = hash.strip_suffix(".patch").unwrap_or(&hash);
        let repo = require_repo(&state, &repo_name).await?;
        let page = load(&state.repo_port, &state.git_port, &repo_name, hash)
            .await
            .map_err(|e| AppError::from_domain(e, Some(repo)))?;
        if patch {
            return Ok((
                [
                    (
                        axum::http::header::CONTENT_TYPE,
                        "text/x-patch; charset=utf-8".to_string(),
                    ),
                    (
                        axum::http::header::CONTENT_DISPOSITION,
                        format!(
                            "attachment; filename=\"{}.patch\"",
                            page.diff.commit.short_hash
                        ),
                    ),
                    (
                        axum::http::header::X_CONTENT_TYPE_OPTIONS,
                        "nosniff".to_string(),
                    ),
                    (
                        axum::http::header::CONTENT_SECURITY_POLICY,
                        "default-src 'none'; sandbox".to_string(),
                    ),
                ],
                page.diff.patch_text,
            )
                .into_response());
        }
        let mut header = RepoHeader::new(&page.repo.name);
        header.status = Some(commit_status(&page.diff.commit.hash));
        header.path = Some(PathRow {
            description: String::new(),
            root_repo_href: None,
            crumbs: Vec::new(),
            ellipsis: false,
            actions: commit_actions(&page),
            current_action: "",
        });
        let content = render_body(&page)?;
        super::render(
            &format!(
                "{} - {} - {}",
                page.repo.name, page.diff.commit.short_hash, page.diff.commit.subject
            ),
            Some(&header),
            None,
            content,
        )
        .map(|html| Html(html).into_response())
    }
    .await
}

pub async fn load<R: RepositoryPort, G: GitPort>(
    repo_port: &R,
    git_port: &G,
    repo_name: &str,
    hash: &str,
) -> Result<CommitPage, DomainError> {
    let repo = repo_port
        .get_repository(repo_name)
        .await?
        .ok_or(DomainError::RepoNotFound)?;
    let rev = repo_port.resolve_rev(repo.id, hash).await?;
    let diff = git_port
        .commit_diff(repo.path.clone(), rev.commit_hash)
        .await?;
    Ok(CommitPage { repo, diff })
}

fn commit_actions(page: &CommitPage) -> Vec<ActionLink> {
    let diff = &page.diff;
    let repo = &page.repo.name;
    let mut actions = vec![ActionLink {
        label: "Browse tree",
        href: path_route(repo, "tree", &diff.commit.hash, ""),
    }];
    if let Some(parent) = diff.commit.parent_hashes.first() {
        actions.push(ActionLink {
            label: "Parent",
            href: path_route(repo, "commit", parent, ""),
        });
    }
    actions.push(ActionLink {
        label: "Raw patch",
        href: path_route(repo, "commit", &format!("{}.patch", diff.commit.hash), ""),
    });
    actions
}

fn render_body(page: &CommitPage) -> Result<String, AppError> {
    let _timer = crate::debug_timing::start("render.body");
    let repo = &page.repo.name;
    let diff = &page.diff;
    let c = &diff.commit;
    let files = diff
        .files
        .iter()
        .enumerate()
        .map(|(idx, file)| {
            let deleted = file.status == "deleted";
            let view_rev = if deleted {
                c.parent_hashes.first().map(String::as_str)
            } else {
                Some(c.hash.as_str())
            };
            let view_path = if deleted {
                file.old_path.as_deref().unwrap_or(&file.path)
            } else {
                &file.path
            };
            ChangedFileRow {
                path: file.path.clone(),
                old_path: file.old_path.clone(),
                status: file.status.clone(),
                additions: file.additions,
                deletions: file.deletions,
                blob_href: view_rev
                    .filter(|_| !file.is_submodule)
                    .map(|rev| path_route(repo, "blob", rev, view_path))
                    .unwrap_or_default(),
                view_label: if file.is_submodule {
                    "Submodule"
                } else if deleted {
                    "View before deletion"
                } else {
                    "View file"
                },
                anchor: format!("diff-{}", idx + 1),
                diff: file
                    .patch_lines
                    .iter()
                    .map(|line| DiffLine {
                        class: diff_line_class(line),
                        text: line.clone(),
                    })
                    .collect(),
                truncated: file.truncated,
            }
        })
        .collect();
    CommitTemplate {
        commit: c.clone(),
        parents: c
            .parent_hashes
            .iter()
            .map(|hash| Parent {
                short: short_hash(hash),
                href: path_route(repo, "commit", hash, ""),
            })
            .collect(),
        message: c
            .message
            .split_once('\n')
            .map(|(_, body)| body.trim().to_string())
            .unwrap_or_default(),
        files_changed: diff.files_changed,
        insertions: diff.insertions,
        deletions: diff.deletions,
        files,
        patch_href: path_route(repo, "commit", &format!("{}.patch", c.hash), ""),
        truncated: diff.truncated,
    }
    .render()
    .map_err(AppError::internal)
}

pub(super) fn diff_line_class(line: &str) -> &'static str {
    const HEADERS: [&str; 11] = [
        "diff --git ",
        "index ",
        "new file mode ",
        "deleted file mode ",
        "old mode ",
        "new mode ",
        "similarity index ",
        "rename from ",
        "rename to ",
        "Binary files ",
        "GIT binary patch",
    ];
    if HEADERS.iter().any(|prefix| line.starts_with(prefix))
        || line.starts_with("--- ")
        || line.starts_with("+++ ")
    {
        "diff-head"
    } else if line.starts_with('+') {
        "diff-add"
    } else if line.starts_with('-') {
        "diff-del"
    } else if line.starts_with("@@") {
        "diff-hunk"
    } else {
        ""
    }
}
