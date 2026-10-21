mod helpers;

use app::page::refs;
use helpers::repo_port;

#[tokio::test]
async fn refs_has_main_branch() {
    let repo = repo_port();
    let view = refs::load(&repo, "test-repo").await.unwrap();
    let branch_names: Vec<&str> = view.branches.iter().map(|b| b.show_name.as_str()).collect();
    assert!(branch_names.contains(&"main"));
}
