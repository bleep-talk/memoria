use batch_recovery_decisions::*;
fn generate_cases() -> Vec<(String, bool)> {
    let mut cases = vec![
        ("text\n".into(), true),
        ("---\n---\n".into(), true),
        ("---\ndescription: [x]\n---\n".into(), false),
        ("---\ndescription: 3\n---\n".into(), false),
        ("---\ndescription: null\n---\n".into(), false),
        ("---\n[]\n---\n".into(), false),
        ("---\nkey: 1\nkey: 2\n---\n".into(), false),
        ("---\nkey: !bad hello\n---\n".into(), false),
        ("---\nkey: [\n---\n".into(), false),
        ("---\ndescription: hi".into(), false),
    ];
    for n in 0..256 {
        for nl in ["\n", "\r\n"] {
            cases.push((
                format!(
                    "---{nl}description: '記憶 {n}'{nl}unknown: {{a: 2}}{nl}---{nl}body {n}{nl}"
                ),
                true,
            ));
        }
    }
    cases
}
#[test]
fn markdown_property() {
    let p = Policy::default();
    for (text, valid) in generate_cases() {
        let parsed = parse_markdown(text.as_bytes(), &p);
        assert_eq!(parsed.is_ok(), valid, "{text:?}: {parsed:?}");
        if let Ok(doc) = parsed {
            assert_eq!(doc.raw().as_bytes(), text.as_bytes());
            assert_eq!(doc.hash(), &hash_content(text.as_bytes()));
        }
    }
    assert!(matches!(
        parse_markdown(&[0xff], &p),
        Err(MemoryError::InvalidUtf8)
    ));
    assert_eq!(
        hash_content(b"abc").as_str(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let tiny = p.with_limits(3, 64, 4, 8).unwrap();
    assert!(parse_markdown(b"four", &tiny).is_err());
}
