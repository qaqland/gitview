mod helpers;

use app::page::tree;
use helpers::{git_port, repo_port};

#[tokio::test]
async fn tree_root_has_my_txt() {
    let repo = repo_port();
    let git = git_port();
    let view = tree::load(&repo, &git, "test-repo", "main", String::new())
        .await
        .unwrap();
    let names: Vec<&str> = view.entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"my.txt"));
}
