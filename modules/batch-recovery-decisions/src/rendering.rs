// RMS declared role: projector
// Module: batch-recovery-decisions
// Machine mode: stateful-transition-machine
// Fill this role body without changing semantics outside RMS spec apply or focused machine structure outside RMS machine apply.
use crate::{parse_markdown, MemoryError, MemoryPath, Policy};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SnapshotEntry {
    Directory(MemoryPath),
    File(MemoryPath, String),
    Unsafe(MemoryPath),
}
impl SnapshotEntry {
    pub fn path(&self) -> &MemoryPath {
        match self {
            Self::Directory(p) | Self::File(p, _) | Self::Unsafe(p) => p,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContextOptions {
    max_bytes: usize,
}
impl Default for ContextOptions {
    fn default() -> Self {
        Self { max_bytes: 32768 }
    }
}
impl ContextOptions {
    pub fn new(max_bytes: usize) -> Self {
        Self { max_bytes }
    }
    pub fn max_bytes(&self) -> usize {
        self.max_bytes
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MemoryContext {
    text: String,
    files: Vec<MemoryPath>,
    byte_len: usize,
}
impl MemoryContext {
    pub fn new(
        entries: &[SnapshotEntry],
        policy: &Policy,
        options: ContextOptions,
    ) -> Result<Self, MemoryError> {
        let result = render_memory(&RenderRequest::Context(
            entries.to_vec(),
            policy.clone(),
            options,
        ))?;
        if let RenderResult::Context(context) = result {
            Ok(context)
        } else {
            Err(MemoryError::InvalidPolicy)
        }
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn files(&self) -> &[MemoryPath] {
        &self.files
    }
    pub fn byte_len(&self) -> usize {
        self.byte_len
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    Directory,
    File,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TreeEntry {
    path: MemoryPath,
    kind: EntryKind,
    description: Option<String>,
}
impl TreeEntry {
    pub fn new(path: MemoryPath, directory: bool, description: Option<String>) -> Self {
        Self {
            path,
            kind: if directory {
                EntryKind::Directory
            } else {
                EntryKind::File
            },
            description,
        }
    }
    pub fn path(&self) -> &MemoryPath {
        &self.path
    }
    pub fn is_directory(&self) -> bool {
        self.kind == EntryKind::Directory
    }
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderRequest {
    Context(Vec<SnapshotEntry>, Policy, ContextOptions),
    Tree(Vec<SnapshotEntry>, Policy, bool),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderResult {
    Context(MemoryContext),
    Tree(Vec<TreeEntry>),
}
#[derive(Serialize)]
struct ContextEntry {
    path: MemoryPath,
    metadata: BTreeMap<String, serde_json::Value>,
    body: String,
}
#[derive(Serialize)]
struct ContextDocument {
    entries: Vec<ContextEntry>,
}

pub fn render_memory(request: &RenderRequest) -> Result<RenderResult, MemoryError> {
    let (entries, policy) = match request {
        RenderRequest::Context(e, p, _) | RenderRequest::Tree(e, p, _) => (e, p),
    };
    if entries.len() > policy.max_entries() {
        return Err(MemoryError::ParserLimitExceeded);
    }
    let mut seen = BTreeSet::new();
    for entry in entries {
        policy.check_directory(entry.path())?;
        if !seen.insert(entry.path()) {
            return Err(MemoryError::DuplicateSnapshotPath(entry.path().to_string()));
        }
        if let SnapshotEntry::Unsafe(path) = entry {
            return Err(MemoryError::UnsupportedEntryKind(path.to_string()));
        }
    }
    match request {
        RenderRequest::Context(_, _, options) => {
            let mut result = Vec::new();
            for entry in entries {
                if let SnapshotEntry::File(path, content) = entry {
                    if path.is_system() && path.is_markdown() {
                        let document = parse_markdown(content.as_bytes(), policy)?;
                        result.push(ContextEntry {
                            path: path.clone(),
                            metadata: document.metadata().clone(),
                            body: document.body().to_owned(),
                        });
                    }
                }
            }
            result.sort_by(|a, b| a.path.cmp(&b.path));
            let files = result.iter().map(|e| e.path.clone()).collect();
            let text = serde_json::to_string(&ContextDocument { entries: result })
                .map_err(|_| MemoryError::ArithmeticOverflow)?;
            if text.len() > options.max_bytes() {
                return Err(MemoryError::ContextTooLarge {
                    required: text.len(),
                    allowed: options.max_bytes(),
                });
            }
            let byte_len = text.len();
            Ok(RenderResult::Context(MemoryContext {
                text,
                files,
                byte_len,
            }))
        }
        RenderRequest::Tree(_, _, descriptions) => {
            let mut tree = BTreeMap::new();
            for root in policy.roots() {
                let path = MemoryPath::new(root)?;
                tree.insert(path.clone(), TreeEntry::new(path, true, None));
            }
            for entry in entries {
                let path = entry.path();
                let mut prefix = String::new();
                let parts: Vec<_> = path.as_str().split('/').collect();
                for part in parts.iter().take(parts.len().saturating_sub(1)) {
                    if !prefix.is_empty() {
                        prefix.push('/');
                    }
                    prefix.push_str(part);
                    let dir = MemoryPath::new(&prefix)?;
                    if tree.get(&dir).is_some_and(|e| !e.is_directory()) {
                        return Err(MemoryError::UnsupportedEntryKind(dir.to_string()));
                    }
                    tree.entry(dir.clone())
                        .or_insert_with(|| TreeEntry::new(dir, true, None));
                }
                let value = match entry {
                    SnapshotEntry::Directory(_) => TreeEntry::new(path.clone(), true, None),
                    SnapshotEntry::File(_, content) => {
                        if !path.is_markdown() {
                            continue;
                        }
                        let description = if *descriptions {
                            parse_markdown(content.as_bytes(), policy)?
                                .description()
                                .map(String::from)
                        } else {
                            None
                        };
                        if tree.get(path).is_some_and(TreeEntry::is_directory) {
                            return Err(MemoryError::UnsupportedEntryKind(path.to_string()));
                        }
                        TreeEntry::new(path.clone(), false, description)
                    }
                    SnapshotEntry::Unsafe(_) => {
                        return Err(MemoryError::UnsupportedEntryKind(path.to_string()))
                    }
                };
                tree.insert(path.clone(), value);
            }
            Ok(RenderResult::Tree(tree.into_values().collect()))
        }
    }
}
