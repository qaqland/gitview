mod helpers;

use app::page::commit;
use helpers::{git_port, repo_port};

#[tokio::test]
async fn commit_has_info_and_diff() {
    let repo = repo_port();
    let git = git_port();
    let page = commit::load(&repo, &git, "test-repo", "main")
        .await
        .unwrap();
    let diff = page.diff;
    assert_eq!(diff.commit.subject, "m-0000020");
    assert_eq!(diff.commit.author, "Test");
    assert!(!diff.files.is_empty());
    assert!(!diff.patch_text.is_empty());
}
