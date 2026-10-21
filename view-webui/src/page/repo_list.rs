use askama::Template;
use axum::{extract::State, response::Html};

use crate::data::RepositoryPort;
use crate::error::{AppError, DomainError};
use crate::format::utc_time_without_zone;
use crate::web::AppState;

#[derive(Clone, Debug)]
pub struct RepoListRow {
    pub name: String,
    pub description: String,
    pub updated: Option<i64>,
}

struct RepoRow {
    href: String,
    name: String,
    description: String,
    updated: String,
}

#[derive(Template)]
#[template(path = "repo_list.html")]
struct RepoListTemplate {
    rows: Vec<RepoRow>,
}

pub async fn handler(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    async {
        let rows = load(&state.repo_port).await.map_err(AppError::internal)?;
        let content = render_body(&rows)?;
        super::render("projects", None, None, content).map(Html)
    }
    .await
}

pub async fn load<R: RepositoryPort>(repo_port: &R) -> Result<Vec<RepoListRow>, DomainError> {
    let repos = repo_port.list_repositories().await?;
    let mut rows = Vec::new();
    for repo in repos {
        let default_rev = repo_port
            .default_rev(&repo)
            .await
            .unwrap_or_else(|_| "master".to_string());
        let updated = repo_port
            .ref_time(repo.id, &default_rev)
            .await
            .ok()
            .flatten();
        let description = read_description(&repo.path);
        rows.push(RepoListRow {
            name: repo.name,
            description,
            updated,
        });
    }
    Ok(rows)
}

fn render_body(rows: &[RepoListRow]) -> Result<String, AppError> {
    let _timer = crate::debug_timing::start("render.body");
    let rows = rows
        .iter()
        .map(|row| RepoRow {
            href: crate::url::repo_route(&row.name),
            name: row.name.clone(),
            description: row.description.clone(),
            updated: row
                .updated
                .map(utc_time_without_zone)
                .unwrap_or_else(|| "--".to_string()),
        })
        .collect();
    RepoListTemplate { rows }
        .render()
        .map_err(AppError::internal)
}

pub(super) fn read_description(repo_path: &str) -> String {
    let _timer = crate::debug_timing::start("fs.read_description");
    let Ok(repo) = git2::Repository::open(repo_path) else {
        return String::new();
    };
    let path = repo.path().join("description");
    let Ok(content) = std::fs::read_to_string(path) else {
        return String::new();
    };
    content.trim().to_string()
}
