use memoria_memory::{ContextOptions, ExpectedContent, MemoryRepo};

fn configure_test_identity(root: &std::path::Path) {
    let git = git2::Repository::open(root).unwrap();
    let mut config = git.config().unwrap();
    config.set_str("user.name", "Memoria Test").unwrap();
    config
        .set_str("user.email", "memoria-test@example.invalid")
        .unwrap();
}

#[test]
fn external_consumer_and_cli() {
    let temp = tempfile::tempdir().unwrap();
    let repo = MemoryRepo::init(temp.path().join("memory")).unwrap();
    repo.write(
        "system/user.md",
        "---\ndescription: User preferences\n---\nThe user prefers Rust.\n",
        ExpectedContent::Absent,
    )
    .unwrap();
    let context = repo.build_context(ContextOptions::default()).unwrap();
    assert!(context.text().contains("The user prefers Rust."));
    assert_eq!(
        repo.tree(false)
            .unwrap()
            .iter()
            .filter(|e| !e.is_directory())
            .count(),
        1
    );
    assert!(!repo.diff().unwrap().is_empty());
}

#[test]
fn commit_keeps_git_index_clean_and_rejects_existing_staging() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("memory");
    let repo = MemoryRepo::init(&root).unwrap();
    configure_test_identity(&root);
    repo.write("knowledge/one.md", "first", ExpectedContent::Absent)
        .unwrap();
    let first = repo.commit("First memory").unwrap();
    assert_eq!(
        repo.log().unwrap()[0].split(' ').next(),
        Some(first.as_str())
    );
    assert!(repo.status().unwrap().is_empty());
    assert!(repo.diff().unwrap().is_empty());
    let git = git2::Repository::open(&root).unwrap();
    repo.write("knowledge/two.md", "second", ExpectedContent::Absent)
        .unwrap();
    let mut index = git.index().unwrap();
    index
        .add_path(std::path::Path::new("knowledge/two.md"))
        .unwrap();
    index.write().unwrap();
    assert!(repo.diff().unwrap().contains("second"));
    assert_eq!(
        repo.commit("Should fail").unwrap_err().code(),
        "staged_changes"
    );
    assert_eq!(git.index().unwrap().len(), 2);
}

#[test]
fn paths_policy_frontmatter_and_exact_context_budget() {
    let temp = tempfile::tempdir().unwrap();
    let repo = MemoryRepo::init(temp.path().join("memory")).unwrap();
    assert_eq!(
        repo.write("system/identity.md", "no", ExpectedContent::Absent)
            .unwrap_err()
            .code(),
        "policy_violation"
    );
    assert_eq!(
        repo.write("../outside.md", "no", ExpectedContent::Absent)
            .unwrap_err()
            .code(),
        "invalid_request"
    );
    assert_eq!(
        repo.write("knowledge/.git/config.md", "no", ExpectedContent::Absent)
            .unwrap_err()
            .code(),
        "invalid_request"
    );
    assert_eq!(
        repo.write(
            "system/bad.md",
            "---\ndescription: [not a string]\n---\nbody",
            ExpectedContent::Absent
        )
        .unwrap_err()
        .code(),
        "invalid_request"
    );
    let source = "---\ndescription: Grüße\nunknown: {answer: 42}\n---\nLine one  \r\nLine two\n";
    repo.write("system/é世界.md", source, ExpectedContent::Absent)
        .unwrap();
    let read = repo.read("system/é世界.md").unwrap();
    assert_eq!(read.content().as_bytes(), source.as_bytes());
    assert_eq!(read.metadata()["unknown"]["answer"], 42);
    let context = repo.build_context(ContextOptions::default()).unwrap();
    assert!(context.text().contains("system/é世界.md"));
    assert!(repo
        .build_context(ContextOptions::new(context.byte_len()))
        .is_ok());
    assert_eq!(
        repo.build_context(ContextOptions::new(context.byte_len() - 1))
            .unwrap_err()
            .code(),
        "context_too_large"
    );
    let tree = repo.tree(true).unwrap();
    assert!(tree
        .iter()
        .any(|entry| entry.path().as_str() == "system/é世界.md"
            && entry.description() == Some("Grüße")));
}

#[test]
fn stale_precondition_and_conflicting_batch_leave_files_unchanged() {
    use memoria_memory::{ContentHash, MemoryPath, Mutation};
    let temp = tempfile::tempdir().unwrap();
    let repo = MemoryRepo::init(temp.path().join("memory")).unwrap();
    repo.write("knowledge/a.md", "one", ExpectedContent::Absent)
        .unwrap();
    let first = repo.read("knowledge/a.md").unwrap();
    repo.write(
        "knowledge/a.md",
        "two",
        ExpectedContent::Hash(first.hash().clone()),
    )
    .unwrap();
    assert_eq!(
        repo.write(
            "knowledge/a.md",
            "three",
            ExpectedContent::Hash(first.hash().clone())
        )
        .unwrap_err()
        .code(),
        "expected_hash_mismatch"
    );
    let a = MemoryPath::new("knowledge/a.md").unwrap();
    let b = MemoryPath::new("knowledge/b.md").unwrap();
    let batch = vec![
        Mutation::Create {
            path: b.clone(),
            content: "new".into(),
        },
        Mutation::Replace {
            path: a,
            content: "stale".into(),
            expected: ContentHash::new("0".repeat(64)).unwrap(),
        },
    ];
    assert_eq!(
        repo.apply(batch).unwrap_err().code(),
        "expected_hash_mismatch"
    );
    assert!(!repo.exists(b.as_str()).unwrap());
    assert_eq!(repo.read("knowledge/a.md").unwrap().content(), "two");
}

#[cfg(unix)]
#[test]
fn symlink_memory_entries_are_rejected() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("memory");
    let repo = MemoryRepo::init(&root).unwrap();
    symlink(temp.path(), root.join("knowledge/link.md")).unwrap();
    assert_eq!(
        repo.read("knowledge/link.md").unwrap_err().code(),
        "unsafe_path"
    );
    assert_eq!(
        repo.write("knowledge/link.md", "x", ExpectedContent::Absent)
            .unwrap_err()
            .code(),
        "unsafe_path"
    );
    assert_eq!(repo.tree(false).unwrap_err().code(), "unsafe_path");
}

#[cfg(unix)]
#[test]
fn symlink_ancestor_is_rejected_without_touching_target() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("memory");
    let outside = temp.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    let repo = MemoryRepo::init(&root).unwrap();
    symlink(&outside, root.join("knowledge/alias")).unwrap();
    assert_eq!(
        repo.write("knowledge/alias/x.md", "secret", ExpectedContent::Absent)
            .unwrap_err()
            .code(),
        "unsafe_path"
    );
    assert!(!outside.join("x.md").exists());
}

#[test]
fn gitlink_submodule_is_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("memory");
    let repo = MemoryRepo::init(&root).unwrap();
    configure_test_identity(&root);
    repo.write("knowledge/one.md", "one", ExpectedContent::Absent)
        .unwrap();
    let commit = repo.commit("First memory").unwrap();
    let git = git2::Repository::open(&root).unwrap();
    let mut index = git.index().unwrap();
    index
        .add(&git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: 0o160000,
            uid: 0,
            gid: 0,
            file_size: 0,
            id: git2::Oid::from_str(&commit).unwrap(),
            flags: 0,
            flags_extended: 0,
            path: b"submodule".to_vec(),
        })
        .unwrap();
    index.write().unwrap();
    assert_eq!(
        MemoryRepo::open(&root).err().unwrap().code(),
        "unsupported_repository"
    );
}

#[test]
fn bare_repository_is_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("bare.git");
    git2::Repository::init_bare(&root).unwrap();
    assert_eq!(
        MemoryRepo::open(&root).err().unwrap().code(),
        "unsupported_repository"
    );
}

#[test]
fn empty_repository_does_not_publish_a_commit() {
    let temp = tempfile::tempdir().unwrap();
    let repo = MemoryRepo::init(temp.path().join("memory")).unwrap();
    assert_eq!(repo.commit("Empty").unwrap_err().code(), "already_exists");
    assert!(repo.log().unwrap().is_empty());
}

#[test]
fn encoded_journal_limit_rejects_before_file_application() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("memory");
    let policy = memoria_memory::Policy::default()
        .with_limits(2 * 1024 * 1024, 2 * 1024 * 1024, 4, 100)
        .unwrap();
    let repo = MemoryRepo::init_with_policy(&root, policy).unwrap();
    let escaped = "\0".repeat(1024 * 1024);
    assert_eq!(
        repo.write("knowledge/big.md", &escaped, ExpectedContent::Absent)
            .unwrap_err()
            .code(),
        "journal_limit_exceeded"
    );
    assert!(!root.join("knowledge/big.md").exists());
    assert!(!root.join(".git/memoria/journal.json").exists());
}
