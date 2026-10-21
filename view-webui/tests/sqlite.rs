mod helpers;

use app::data::RepositoryPort;
use helpers::repo_port;

#[tokio::test]
async fn get_repository_found() {
    let repo = repo_port();
    let rec = repo.get_repository("test-repo").await.unwrap().unwrap();
    assert_eq!(rec.name, "test-repo");
}
