// RMS declared role: content_hash
// Module: batch-recovery-decisions
// Machine mode: stateful-transition-machine
// Fill this role body without changing semantics outside RMS spec apply or focused machine structure outside RMS machine apply.
use crate::MemoryError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ContentHash(String);

impl ContentHash {
    pub fn new(value: impl Into<String>) -> Result<Self, MemoryError> {
        let value = value.into();
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(MemoryError::InvalidHash);
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for ContentHash {
    type Error = MemoryError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<ContentHash> for String {
    fn from(hash: ContentHash) -> String {
        hash.0
    }
}
pub fn hash_content(bytes: &[u8]) -> ContentHash {
    ContentHash(format!("{:x}", Sha256::digest(bytes)))
}
