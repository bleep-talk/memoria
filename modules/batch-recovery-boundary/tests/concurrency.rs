use memoria_memory::{ContentHash, ExpectedContent, MemoryRepo};
use std::process::Command;
#[test]
fn process_worker() {
    let Ok(root) = std::env::var("MEMORIA_WORKER_ROOT") else {
        return;
    };
    let repo = MemoryRepo::open(root).unwrap();
    let expected = match std::env::var("MEMORIA_WORKER_EXPECTED") {
        Ok(hash) => ContentHash::new(&hash).unwrap(),
        Err(_) => repo.read("knowledge/test.md").unwrap().hash().clone(),
    };
    let result = repo.write(
        "knowledge/test.md",
        "after",
        ExpectedContent::Hash(expected),
    );
    if std::env::var("MEMORIA_WORKER_EXPECTED").is_ok() {
        match result {
            Ok(_) => return,
            Err(error) if error.code() == "expected_hash_mismatch" => std::process::exit(3),
            Err(error) => panic!("unexpected worker error: {error}"),
        }
    }
    result.unwrap();
}

#[test]
fn commit_worker() {
    let Ok(root) = std::env::var("MEMORIA_COMMIT_WORKER_ROOT") else {
        return;
    };
    let repo = MemoryRepo::open(root).unwrap();
    repo.commit("Remember crash recovery").unwrap();
}

#[test]
fn delete_worker() {
    let Ok(root) = std::env::var("MEMORIA_DELETE_WORKER_ROOT") else {
        return;
    };
    let repo = MemoryRepo::open(root).unwrap();
    let hash = repo.read("knowledge/test.md").unwrap().hash().clone();
    repo.delete("knowledge/test.md", hash).unwrap();
}

#[test]
fn recovery_worker() {
    let Ok(root) = std::env::var("MEMORIA_RECOVERY_WORKER_ROOT") else {
        return;
    };
    MemoryRepo::open(root).unwrap();
}

#[test]
fn delete_and_rollback_crashes_recover_without_losing_memory() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("memory");
    let repo = MemoryRepo::init(&root).unwrap();
    repo.write("knowledge/test.md", "before", ExpectedContent::Absent)
        .unwrap();
    drop(repo);

    let deleted = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "delete_worker"])
        .env("MEMORIA_DELETE_WORKER_ROOT", &root)
        .env("MEMORIA_TEST_CRASH", "delete_after_unlink")
        .status()
        .unwrap();
    assert!(!deleted.success());

    let rollback = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "recovery_worker"])
        .env("MEMORIA_RECOVERY_WORKER_ROOT", &root)
        .env("MEMORIA_TEST_CRASH", "rollback_after_change")
        .status()
        .unwrap();
    assert!(!rollback.success());

    let repo = MemoryRepo::open(&root).unwrap();
    assert_eq!(repo.read("knowledge/test.md").unwrap().content(), "before");
    assert!(!root.join(".git/memoria/journal.json").exists());
}

#[test]
fn commit_publication_recovers_after_process_death() {
    for (point, published) in [("commit_after_intent", false), ("commit_after_ref", true)] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("memory");
        let repo = MemoryRepo::init(&root).unwrap();
        let git = git2::Repository::open(&root).unwrap();
        let mut config = git.config().unwrap();
        config.set_str("user.name", "Memoria Test").unwrap();
        config
            .set_str("user.email", "memoria@example.invalid")
            .unwrap();
        repo.write("knowledge/test.md", "before", ExpectedContent::Absent)
            .unwrap();
        drop(repo);
        let status = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "commit_worker"])
            .env("MEMORIA_COMMIT_WORKER_ROOT", &root)
            .env("MEMORIA_TEST_CRASH", point)
            .status()
            .unwrap();
        assert!(!status.success(), "crash point did not fire: {point}");
        let repo = MemoryRepo::open(&root).unwrap();
        assert_eq!(!repo.log().unwrap().is_empty(), published, "{point}");
        assert!(!root.join(".git/memoria/commit-intent.json").exists());
        if !published {
            repo.commit("Remember crash recovery").unwrap();
        }
        assert_eq!(repo.log().unwrap().len(), 1);
    }
}

#[test]
fn commit_recovery_preserves_external_staging() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("memory");
    let repo = MemoryRepo::init(&root).unwrap();
    let git = git2::Repository::open(&root).unwrap();
    let mut config = git.config().unwrap();
    config.set_str("user.name", "Memoria Test").unwrap();
    config
        .set_str("user.email", "memoria@example.invalid")
        .unwrap();
    repo.write("knowledge/test.md", "before", ExpectedContent::Absent)
        .unwrap();
    drop(repo);
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "commit_worker"])
        .env("MEMORIA_COMMIT_WORKER_ROOT", &root)
        .env("MEMORIA_TEST_CRASH", "commit_after_ref")
        .status()
        .unwrap();
    assert!(!status.success());
    std::fs::write(root.join("unmanaged.txt"), "external").unwrap();
    let mut index = git.index().unwrap();
    index
        .add_path(std::path::Path::new("unmanaged.txt"))
        .unwrap();
    index.write().unwrap();
    assert_eq!(
        MemoryRepo::open(&root).err().unwrap().code(),
        "staged_changes"
    );
    assert!(root.join(".git/memoria/commit-intent.json").exists());
    assert_eq!(
        std::fs::read_to_string(root.join("unmanaged.txt")).unwrap(),
        "external"
    );
}
#[test]
fn crash_restart_recovers() {
    for (point, expected) in [
        ("journal_before_persist", "before"),
        ("journal_before_apply", "before"),
        ("replace_after_rename", "before"),
        ("apply_complete_before_marker", "before"),
        ("completion_after_flush", "after"),
        ("cleanup_before_delete", "after"),
        ("cleanup_after_delete", "after"),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("memory");
        let repo = MemoryRepo::init(&root).unwrap();
        repo.write("knowledge/test.md", "before", ExpectedContent::Absent)
            .unwrap();
        drop(repo);
        let status = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "process_worker", "--nocapture"])
            .env("MEMORIA_WORKER_ROOT", &root)
            .env("MEMORIA_TEST_CRASH", point)
            .status()
            .unwrap();
        assert!(!status.success(), "crash point did not fire: {point}");
        let repo = MemoryRepo::open(&root).unwrap();
        assert_eq!(repo.read("knowledge/test.md").unwrap().content(), expected);
        drop(repo);
        assert_eq!(
            MemoryRepo::open(&root)
                .unwrap()
                .read("knowledge/test.md")
                .unwrap()
                .content(),
            expected
        );
    }
}
#[test]
fn two_process_repository_serialization() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("memory");
    let repo = MemoryRepo::init(&root).unwrap();
    repo.write("knowledge/test.md", "before", ExpectedContent::Absent)
        .unwrap();
    let expected = repo
        .read("knowledge/test.md")
        .unwrap()
        .hash()
        .as_str()
        .to_owned();
    let mut a = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "process_worker"])
        .env("MEMORIA_WORKER_ROOT", &root)
        .env("MEMORIA_WORKER_EXPECTED", &expected)
        .spawn()
        .unwrap();
    let mut b = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "process_worker"])
        .env("MEMORIA_WORKER_ROOT", &root)
        .env("MEMORIA_WORKER_EXPECTED", &expected)
        .spawn()
        .unwrap();
    let a = a.wait().unwrap();
    let b = b.wait().unwrap();
    assert!(
        a.success() ^ b.success(),
        "exactly one stale-precondition write must succeed"
    );
    assert_eq!(if a.success() { b.code() } else { a.code() }, Some(3));
    assert_eq!(repo.read("knowledge/test.md").unwrap().content(), "after");
}

#[test]
fn external_edit_conflict_preserves_memory_and_journal() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("memory");
    let repo = MemoryRepo::init(&root).unwrap();
    repo.write("knowledge/test.md", "before", ExpectedContent::Absent)
        .unwrap();
    drop(repo);
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "process_worker"])
        .env("MEMORIA_WORKER_ROOT", &root)
        .env("MEMORIA_TEST_CRASH", "journal_before_apply")
        .status()
        .unwrap();
    assert!(!status.success());
    std::fs::write(root.join("knowledge/test.md"), "external").unwrap();
    assert_eq!(
        MemoryRepo::open(&root).err().unwrap().code(),
        "recovery_conflict"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("knowledge/test.md")).unwrap(),
        "external"
    );
    assert!(root.join(".git/memoria/journal.json").exists());
}

#[test]
fn foreign_repository_identity_cannot_replay_journal() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("memory");
    let repo = MemoryRepo::init(&root).unwrap();
    repo.write("knowledge/test.md", "before", ExpectedContent::Absent)
        .unwrap();
    drop(repo);
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "process_worker"])
        .env("MEMORIA_WORKER_ROOT", &root)
        .env("MEMORIA_TEST_CRASH", "journal_before_apply")
        .status()
        .unwrap();
    assert!(!status.success());
    let journal = root.join(".git/memoria/journal.json");
    let evidence = std::fs::read(&journal).unwrap();
    std::fs::write(
        root.join(".git/memoria/repository-id"),
        "11111111-1111-4111-8111-111111111111",
    )
    .unwrap();
    assert_eq!(
        MemoryRepo::open(&root).err().unwrap().code(),
        "invalid_journal"
    );
    assert_eq!(std::fs::read(journal).unwrap(), evidence);
    assert_eq!(
        std::fs::read_to_string(root.join("knowledge/test.md")).unwrap(),
        "before"
    );
}

#[test]
fn completed_journal_preserves_external_edit_conflict() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("memory");
    let repo = MemoryRepo::init(&root).unwrap();
    repo.write("knowledge/test.md", "before", ExpectedContent::Absent)
        .unwrap();
    drop(repo);
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "process_worker"])
        .env("MEMORIA_WORKER_ROOT", &root)
        .env("MEMORIA_TEST_CRASH", "completion_after_flush")
        .status()
        .unwrap();
    assert!(!status.success());
    std::fs::write(root.join("knowledge/test.md"), "external").unwrap();
    assert_eq!(
        MemoryRepo::open(&root).err().unwrap().code(),
        "recovery_conflict"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("knowledge/test.md")).unwrap(),
        "external"
    );
    assert!(root.join(".git/memoria/journal.json").exists());
}
