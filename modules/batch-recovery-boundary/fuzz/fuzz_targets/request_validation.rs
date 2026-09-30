#![no_main]

use batch_recovery_decisions::Policy;
use libfuzzer_sys::fuzz_target;

#[path = "../../src/parser.rs"]
mod parser;

fn generate_request(bytes: &[u8]) -> Option<&str> {
    std::str::from_utf8(bytes).ok()
}

fn fuzz_one(bytes: &[u8]) {
    if let Some(path) = generate_request(bytes) {
        let policy = Policy::default();
        for mutation in [false, true] {
            if let Ok(parsed) = parser::parse_request(path, &policy, mutation) {
                assert_eq!(parsed.as_str(), path);
                assert!(matches!(parsed.root(), "system" | "knowledge" | "skills" | "conversations"));
                assert!(path.ends_with(".md"));
                assert!(path.contains('/'));
                assert!(!path.contains(['\\', ':']));
                assert!(!path.chars().any(char::is_control));
                assert!(path.len() <= 4096);
                assert!(path.split('/').count() <= 64);
                assert!(path.split('/').all(|part| !part.is_empty()
                    && part != "."
                    && part != ".."
                    && !part.eq_ignore_ascii_case(".git")));
                if mutation {
                    assert!(!path.eq_ignore_ascii_case("system/identity.md"));
                    assert!(!path.eq_ignore_ascii_case("system/policy.md"));
                }
            }
        }
    }
}

fuzz_target!(|bytes: &[u8]| fuzz_one(bytes));
