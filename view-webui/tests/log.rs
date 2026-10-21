mod helpers;

use app::page::history;
use helpers::{git_port, repo_port};

#[tokio::test]
async fn log_with_path_filters_to_my_txt() {
    let repo = repo_port();
    let git = git_port();
    let page = history::load(
        &repo,
        &git,
        "test-repo",
        "main",
        Some("my.txt".into()),
        None,
        30,
    )
    .await
    .unwrap();
    assert_eq!(page.path, Some("my.txt".into()));
    let subjects: Vec<&str> = page
        .log
        .entries
        .iter()
        .map(|e| e.subject.as_str())
        .collect();
    assert!(subjects.contains(&"m-0000020"));
    assert!(subjects.contains(&"m-0000010"));
    assert!(!subjects.contains(&"m-0000019"));
}

#[tokio::test]
async fn log_paginates_with_next_after() {
    let repo = repo_port();
    let git = git_port();
    let page1 = history::load(&repo, &git, "test-repo", "main", None, None, 5)
        .await
        .unwrap();
    assert_eq!(page1.log.entries.len(), 5);
    assert!(page1.log.next_after.is_some());

    let page2 = history::load(
        &repo,
        &git,
        "test-repo",
        "main",
        None,
        page1.log.next_after.as_deref(),
        5,
    )
    .await
    .unwrap();
    assert_eq!(page2.log.entries.len(), 5);
    let hashes1: Vec<&str> = page1.log.entries.iter().map(|e| e.hash.as_str()).collect();
    let hashes2: Vec<&str> = page2.log.entries.iter().map(|e| e.hash.as_str()).collect();
    assert!(hashes1.iter().all(|h| !hashes2.contains(h)));
}
