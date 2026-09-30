use batch_recovery_decisions::*;
fn generate_cases() -> Vec<Vec<SnapshotEntry>> {
    (0..128)
        .map(|seed| {
            (0..8)
                .map(|i| {
                    SnapshotEntry::File(
                        MemoryPath::new(&format!(
                            "system/{}-{}.md",
                            ["a", "z", "é", "世界"][i % 4],
                            i
                        ))
                        .unwrap(),
                        format!("---\ndescription: 'memory {seed}-{i}'\n---\ntext \\\" {seed}"),
                    )
                })
                .collect()
        })
        .collect()
}
#[test]
fn rendering_property() {
    let policy = Policy::default();
    for mut entries in generate_cases() {
        let full = MemoryContext::new(&entries, &policy, ContextOptions::default()).unwrap();
        entries.reverse();
        assert_eq!(
            full,
            MemoryContext::new(&entries, &policy, ContextOptions::default()).unwrap()
        );
        let json: serde_json::Value = serde_json::from_str(full.text()).unwrap();
        let rows = json["entries"].as_array().unwrap();
        assert_eq!(rows.len(), entries.len());
        let paths = rows
            .iter()
            .map(|e| e["path"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert!(paths.windows(2).all(|w| w[0].as_bytes() < w[1].as_bytes()));
        for row in rows {
            assert!(row["metadata"]["description"].is_string());
            assert!(row["body"].as_str().unwrap().starts_with("text"));
        }
        assert!(
            MemoryContext::new(&entries, &policy, ContextOptions::new(full.byte_len())).is_ok()
        );
        assert!(matches!(
            MemoryContext::new(&entries, &policy, ContextOptions::new(full.byte_len() - 1)),
            Err(MemoryError::ContextTooLarge { .. })
        ));
        entries.push(entries[0].clone());
        assert!(render_memory(&RenderRequest::Context(
            entries,
            policy.clone(),
            ContextOptions::default()
        ))
        .is_err());
    }
    let RenderResult::Tree(tree) =
        render_memory(&RenderRequest::Tree(vec![], policy.clone(), false)).unwrap()
    else {
        panic!()
    };
    assert_eq!(tree.len(), 4);
    assert!(tree.iter().all(TreeEntry::is_directory));
    let empty = MemoryContext::new(&[], &policy, ContextOptions::default()).unwrap();
    assert_eq!(empty.text(), "{\"entries\":[]}");
}
