use batch_recovery_decisions::*;
fn generate_cases() -> Vec<String> {
    let mut cases = vec![
        "../x",
        "/etc/passwd",
        "system/../x.md",
        ".git/config",
        "system/.GIT/config",
        "system//a.md",
        "system/./a.md",
        "C:/a.md",
        "system\\a.md",
        "system/a\0.md",
        "",
        "system/user.md",
        "knowledge/世界.md",
    ]
    .into_iter()
    .map(String::from)
    .collect::<Vec<_>>();
    for i in 0..512 {
        cases.push(format!(
            "knowledge/project-{i}/note-{}.md",
            (i * 7919) % 104729
        ));
    }
    cases
}
#[test]
fn path_policy_property() {
    let policy = Policy::default();
    for raw in generate_cases() {
        let lexical = !raw.is_empty()
            && raw.len() <= 4096
            && !raw.contains(['\\', ':'])
            && !raw.chars().any(char::is_control)
            && raw
                .split('/')
                .all(|s| !s.is_empty() && s != "." && s != ".." && !s.eq_ignore_ascii_case(".git"));
        assert_eq!(MemoryPath::new(&raw).is_ok(), lexical, "{raw:?}");
        if let Ok(path) = construct_memory_values(&raw, &policy, false) {
            assert_eq!(path.as_str(), raw);
        }
    }
    for raw in [
        "system/identity.md",
        "system/policy.md",
        "system/IDENTITY.md",
    ] {
        assert!(matches!(
            construct_memory_values(raw, &policy, true),
            Err(MemoryError::ProtectedMutation(_))
        ));
        assert!(construct_memory_values(raw, &policy, false).is_ok());
    }
    let custom = Policy::new(["private".into()], []).unwrap();
    assert!(construct_memory_values("private/a.md", &custom, true).is_ok());
    assert!(construct_memory_values("knowledge/a.md", &custom, true).is_err());
    let invalid = r#""../secret""#;
    assert!(serde_json::from_str::<MemoryPath>(invalid).is_err());
}
