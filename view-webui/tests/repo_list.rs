mod helpers;

use app::page::repo_list;
use helpers::repo_port;

#[tokio::test]
async fn repo_list_row_has_fields() {
    let repo = repo_port();
    let rows = repo_list::load(&repo).await.unwrap();
    let row = rows.iter().find(|r| r.name == "test-repo").unwrap();
    assert_eq!(row.name, "test-repo");
    assert!(row.updated.is_some());
}
