// RMS declared role: representation
// Module: batch-recovery-decisions
// Machine mode: stateful-transition-machine
// Fill this role body without changing semantics outside RMS spec apply or focused machine structure outside RMS machine apply.
use crate::{MemoryError, Policy};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A literal repository-relative path. No URL decoding or normalization occurs.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct MemoryPath(String);

impl MemoryPath {
    pub fn new(value: &str) -> Result<Self, MemoryError> {
        let value = value.to_owned();
        if value.is_empty()
            || value.len() > 4096
            || value.contains(['\\', ':'])
            || value.chars().any(char::is_control)
            || value
                .split('/')
                .any(|s| s.is_empty() || s == "." || s == ".." || s.eq_ignore_ascii_case(".git"))
            || value.split('/').count() > 64
        {
            return Err(MemoryError::InvalidPath(value));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn root(&self) -> &str {
        self.0.split('/').next().unwrap_or("")
    }
    pub fn is_markdown(&self) -> bool {
        self.0.contains('/') && self.0.ends_with(".md")
    }
    pub fn is_system(&self) -> bool {
        self.0.starts_with("system/")
    }
}

impl TryFrom<String> for MemoryPath {
    type Error = MemoryError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(&value)
    }
}
impl From<MemoryPath> for String {
    fn from(path: MemoryPath) -> String {
        path.0
    }
}
impl fmt::Display for MemoryPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Construct a path and enforce the supplied host policy.
pub fn construct_memory_values(
    raw: &str,
    policy: &Policy,
    mutation: bool,
) -> Result<MemoryPath, MemoryError> {
    let path = MemoryPath::new(raw)?;
    if mutation {
        policy.check_write(&path)?;
    } else {
        policy.check_read(&path)?;
    }
    Ok(path)
}
