// RMS declared role: parser
// Module: batch-recovery-decisions
// Machine mode: stateful-transition-machine
// Fill this role body without changing semantics outside RMS spec apply or focused machine structure outside RMS machine apply.
use crate::{hash_content, ContentHash, MemoryError, Policy};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MarkdownDocument {
    raw: String,
    metadata: BTreeMap<String, Value>,
    body: String,
    hash: ContentHash,
}

impl MarkdownDocument {
    pub fn new(bytes: &[u8], policy: &Policy) -> Result<Self, MemoryError> {
        parse_markdown(bytes, policy)
    }
    pub fn raw(&self) -> &str {
        &self.raw
    }
    pub fn body(&self) -> &str {
        &self.body
    }
    pub fn metadata(&self) -> &BTreeMap<String, Value> {
        &self.metadata
    }
    pub fn hash(&self) -> &ContentHash {
        &self.hash
    }
    pub fn description(&self) -> Option<&str> {
        self.metadata.get("description").and_then(Value::as_str)
    }
}

fn yaml_value(value: serde_yaml_ng::Value, depth: usize) -> Result<Value, MemoryError> {
    use serde_yaml_ng::Value as Y;
    if depth > 32 {
        return Err(MemoryError::ParserLimitExceeded);
    }
    Ok(match value {
        Y::Null => Value::Null,
        Y::Bool(b) => Value::Bool(b),
        Y::Number(n) => {
            serde_json::to_value(n).map_err(|e| MemoryError::MalformedYaml(e.to_string()))?
        }
        Y::String(s) => Value::String(s),
        Y::Sequence(items) => Value::Array(
            items
                .into_iter()
                .map(|v| {
                    yaml_value(
                        v,
                        depth
                            .checked_add(1)
                            .ok_or(MemoryError::ArithmeticOverflow)?,
                    )
                })
                .collect::<Result<_, _>>()?,
        ),
        Y::Mapping(map) => {
            let mut result = serde_json::Map::new();
            for (key, value) in map {
                let Y::String(key) = key else {
                    return Err(MemoryError::MalformedYaml(
                        "metadata keys must be strings".into(),
                    ));
                };
                result.insert(
                    key,
                    yaml_value(
                        value,
                        depth
                            .checked_add(1)
                            .ok_or(MemoryError::ArithmeticOverflow)?,
                    )?,
                );
            }
            Value::Object(result)
        }
        Y::Tagged(_) => {
            return Err(MemoryError::MalformedYaml(
                "custom YAML tags are unsupported".into(),
            ))
        }
    })
}

pub fn parse_markdown(bytes: &[u8], policy: &Policy) -> Result<MarkdownDocument, MemoryError> {
    if bytes.len() > policy.max_file_bytes() {
        return Err(MemoryError::ParserLimitExceeded);
    }
    let raw = std::str::from_utf8(bytes).map_err(|_| MemoryError::InvalidUtf8)?;
    let mut metadata = BTreeMap::new();
    let mut body = raw;
    let first = raw.split_inclusive('\n').next().unwrap_or("");
    if first.trim_end_matches(['\r', '\n']) == "---" {
        let mut end = None;
        let mut offset = first.len();
        for line in raw[offset..].split_inclusive('\n') {
            if line.trim_end_matches(['\r', '\n']) == "---" {
                end = Some((
                    offset,
                    offset
                        .checked_add(line.len())
                        .ok_or(MemoryError::ArithmeticOverflow)?,
                ));
                break;
            }
            offset = offset
                .checked_add(line.len())
                .ok_or(MemoryError::ArithmeticOverflow)?;
        }
        let (yaml_end, body_start) = end.ok_or_else(|| {
            MemoryError::MalformedYaml("frontmatter has no closing delimiter".into())
        })?;
        let yaml = &raw[first.len()..yaml_end];
        if yaml.len() > 64 * 1024 {
            return Err(MemoryError::ParserLimitExceeded);
        }
        if !yaml.trim().is_empty() {
            let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml)
                .map_err(|e| MemoryError::MalformedYaml(e.to_string()))?;
            let Value::Object(map) = yaml_value(parsed, 0)? else {
                return Err(MemoryError::MalformedYaml(
                    "frontmatter must be a mapping".into(),
                ));
            };
            metadata.extend(map);
        }
        if metadata.get("description").is_some_and(|v| !v.is_string()) {
            return Err(MemoryError::InvalidDescription);
        }
        body = &raw[body_start..];
    }
    Ok(MarkdownDocument {
        raw: raw.to_owned(),
        metadata,
        body: body.to_owned(),
        hash: hash_content(bytes),
    })
}
