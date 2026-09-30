// RMS declared role: representation
// Module: batch-recovery-decisions
// Machine mode: stateful-transition-machine
// Fill this role body without changing semantics outside RMS spec apply or focused machine structure outside RMS machine apply.
use crate::{MemoryError, MemoryPath};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Policy {
    roots: BTreeSet<String>,
    protected: BTreeSet<MemoryPath>,
    max_file_bytes: usize,
    max_batch_bytes: usize,
    max_operations: usize,
    max_entries: usize,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            roots: ["system", "knowledge", "skills", "conversations"]
                .into_iter()
                .map(String::from)
                .collect(),
            protected: ["system/identity.md", "system/policy.md"]
                .into_iter()
                .filter_map(|p| MemoryPath::new(p).ok())
                .collect(),
            max_file_bytes: 4 * 1024 * 1024,
            max_batch_bytes: 32 * 1024 * 1024,
            max_operations: 128,
            max_entries: 10000,
        }
    }
}

impl Policy {
    pub fn new(
        roots: impl IntoIterator<Item = String>,
        protected: impl IntoIterator<Item = MemoryPath>,
    ) -> Result<Self, MemoryError> {
        let roots: BTreeSet<_> = roots.into_iter().collect();
        if roots.is_empty()
            || roots
                .iter()
                .any(|s| s.contains('/') || MemoryPath::new(s).is_err())
        {
            return Err(MemoryError::InvalidPolicy);
        }
        let protected: BTreeSet<_> = protected.into_iter().collect();
        if protected
            .iter()
            .any(|p| !roots.contains(p.root()) || !p.is_markdown())
        {
            return Err(MemoryError::InvalidPolicy);
        }
        Ok(Self {
            roots,
            protected,
            ..Self::default()
        })
    }
    pub fn roots(&self) -> &BTreeSet<String> {
        &self.roots
    }
    pub fn max_file_bytes(&self) -> usize {
        self.max_file_bytes
    }
    pub fn max_batch_bytes(&self) -> usize {
        self.max_batch_bytes
    }
    pub fn max_operations(&self) -> usize {
        self.max_operations
    }
    pub fn max_entries(&self) -> usize {
        self.max_entries
    }
    pub fn with_limits(
        mut self,
        file_bytes: usize,
        batch_bytes: usize,
        operations: usize,
        entries: usize,
    ) -> Result<Self, MemoryError> {
        if file_bytes == 0
            || batch_bytes == 0
            || operations == 0
            || entries == 0
            || file_bytes > 64 * 1024 * 1024
            || batch_bytes > 256 * 1024 * 1024
            || operations > 4096
            || entries > 100000
        {
            return Err(MemoryError::InvalidPolicy);
        }
        self.max_file_bytes = file_bytes;
        self.max_batch_bytes = batch_bytes;
        self.max_operations = operations;
        self.max_entries = entries;
        Ok(self)
    }
    pub fn check_directory(&self, path: &MemoryPath) -> Result<(), MemoryError> {
        if !self.roots.contains(path.root()) {
            return Err(MemoryError::InvalidPath(path.to_string()));
        }
        Ok(())
    }
    pub fn check_read(&self, path: &MemoryPath) -> Result<(), MemoryError> {
        self.check_directory(path)?;
        if !path.is_markdown() {
            return Err(MemoryError::InvalidPath(path.to_string()));
        }
        Ok(())
    }
    pub fn check_write(&self, path: &MemoryPath) -> Result<(), MemoryError> {
        self.check_read(path)?;
        if self
            .protected
            .iter()
            .any(|p| p.as_str().eq_ignore_ascii_case(path.as_str()))
        {
            return Err(MemoryError::ProtectedMutation(path.to_string()));
        }
        Ok(())
    }
}
