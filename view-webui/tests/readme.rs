use app::data::git::PREVIEW_LIMIT;
use app::data::{GitPort, GitRepository, ResolvedRev};
use git2::{IndexAddOption, Repository, Signature};
use std::fs;

#[tokio::test]
async fn readmes_are_local_versioned_and_bounded() {
    let temp = tempfile::tempdir().unwrap();
    let repo = Repository::init(temp.path()).unwrap();
    for dir in ["nested", "plain", "binary", "large", "empty", "linked"] {
        fs::create_dir(temp.path().join(dir)).unwrap();
        fs::write(temp.path().join(dir).join("file.txt"), "file\n").unwrap();
    }
    for (path, text) in [
        ("README.md", "# Root\n"),
        ("nested/ReadMe.MD", "# Nested\n"),
        ("nested/README.markdown", "# Lower priority\n"),
        ("nested/README", "Plain fallback\n"),
        ("plain/ReadMe", "# Not Markdown\n<script>unsafe</script>\n"),
        ("binary/README.md", "invalid\0text"),
    ] {
        fs::write(temp.path().join(path), text).unwrap();
    }
    fs::write(
        temp.path().join("large/README.md"),
        vec![b'x'; PREVIEW_LIMIT + 1],
    )
    .unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("../README.md", temp.path().join("linked/README.md")).unwrap();
    let mut index = repo.index().unwrap();
    index.add_all(["*"], IndexAddOption::DEFAULT, None).unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let signature = Signature::now("Test", "test@example.com").unwrap();
    let hash = repo
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "README fixtures",
            &tree,
            &[],
        )
        .unwrap();
    // Working-tree changes must never affect the requested commit's preview.
    fs::write(temp.path().join("nested/ReadMe.MD"), "# Uncommitted\n").unwrap();
    let git = GitRepository::new();
    for (directory, expected_path, expected_text) in [
        ("", "README.md", "Root"),
        ("nested", "nested/ReadMe.MD", "Nested"),
        ("plain", "plain/ReadMe", "# Not Markdown"),
    ] {
        let readme = git
            .read_readme(
                temp.path().to_str().unwrap().into(),
                ResolvedRev {
                    input: "main".into(),
                    commit_hash: hash.to_string(),
                },
                "repo".into(),
                directory.into(),
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(readme.path, expected_path);
        let html = readme.html.unwrap();
        assert!(html.contains(expected_text), "{html}");
        assert!(!html.contains("Uncommitted"));
        assert!(!html.contains("<script>"));
    }
    for directory in ["binary", "large", "empty", "linked"] {
        let readme = git
            .read_readme(
                temp.path().to_str().unwrap().into(),
                ResolvedRev {
                    input: "main".into(),
                    commit_hash: hash.to_string(),
                },
                "repo".into(),
                directory.into(),
            )
            .await
            .unwrap();
        if matches!(directory, "binary" | "large") {
            assert!(readme.unwrap().html.is_none());
        } else {
            assert!(readme.is_none());
        }
    }
}
