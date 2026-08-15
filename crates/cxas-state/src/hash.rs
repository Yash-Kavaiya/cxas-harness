// Copyright 2026 The cxas-harness Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::StateError;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

/// SHA-256 digest of canonicalized app content.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StateHash(pub [u8; 32]);

impl StateHash {
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
}

impl fmt::Display for StateHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

/// Per-path content hashes plus a Merkle-style root hash of the tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppTree {
    pub files: BTreeMap<PathBuf, StateHash>,
    pub root_hash: StateHash,
}

impl AppTree {
    pub fn empty() -> Self {
        Self {
            files: BTreeMap::new(),
            root_hash: hash_bytes(&[]),
        }
    }
}

pub fn hash_bytes(bytes: &[u8]) -> StateHash {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    StateHash(hasher.finalize().into())
}

/// Walk `root`, skip `.git/` and `target/`, and hash each file as `path + NUL + bytes`.
///
/// A `tools/` entry whose canonical path leaves `root` is `StateError::PathEscape`
/// (Phase 4 tool-deletion sync must not follow escaped links).
pub fn hash_app_dir(root: &Path) -> Result<AppTree, StateError> {
    let root_canon = root.canonicalize()?;
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();

    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name();
            name != ".git" && name != "target"
        });

    for entry in walker {
        let entry = entry.map_err(walk_err)?;
        let path = entry.path();
        let rel = match path.strip_prefix(root) {
            Ok(rel) => rel,
            Err(_) => match path.strip_prefix(&root_canon) {
                Ok(rel) => rel,
                Err(_) => return Err(StateError::PathEscape),
            },
        };
        if rel.as_os_str().is_empty() {
            continue;
        }
        if relative_is_tools(rel) {
            let canon = path.canonicalize()?;
            if !contained_in(&canon, &root_canon) {
                return Err(StateError::PathEscape);
            }
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let unix = to_unix_path(rel);
        let raw = std::fs::read(path)?;
        let content = canonicalize_contents(rel, &raw);
        files.insert(unix, content);
    }

    let mut hashed = BTreeMap::new();
    let mut root_hasher = Sha256::new();
    for (unix, content) in &files {
        let mut file_hasher = Sha256::new();
        file_hasher.update(unix.as_bytes());
        file_hasher.update([0u8]);
        file_hasher.update(content);
        hashed.insert(
            PathBuf::from(unix),
            StateHash(file_hasher.finalize().into()),
        );

        root_hasher.update(unix.as_bytes());
        root_hasher.update([0u8]);
        root_hasher.update(content);
    }

    let root_hash = if files.is_empty() {
        hash_bytes(&[])
    } else {
        StateHash(root_hasher.finalize().into())
    };

    Ok(AppTree {
        files: hashed,
        root_hash,
    })
}

fn relative_is_tools(rel: &Path) -> bool {
    matches!(
        rel.components().next(),
        Some(Component::Normal(name)) if name == "tools"
    )
}

fn to_unix_path(rel: &Path) -> String {
    let mut out = String::new();
    for (i, part) in rel.iter().enumerate() {
        if i > 0 {
            out.push('/');
        }
        out.push_str(&part.to_string_lossy());
    }
    out.replace('\\', "/")
}

fn contained_in(path: &Path, root: &Path) -> bool {
    if path.starts_with(root) {
        return true;
    }
    let path_n = strip_verbatim(&path.to_string_lossy());
    let root_n = strip_verbatim(&root.to_string_lossy());
    path_n == root_n
        || path_n.starts_with(&(root_n.clone() + "/"))
        || path_n.starts_with(&(root_n + "\\"))
}

fn strip_verbatim(s: &str) -> String {
    s.strip_prefix(r"\\?\").unwrap_or(s).to_string()
}

fn canonicalize_contents(rel: &Path, bytes: &[u8]) -> Vec<u8> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return bytes.to_vec();
    };
    let lf = text.replace("\r\n", "\n").replace('\r', "\n");
    match rel.extension().and_then(|e| e.to_str()) {
        Some("yaml") | Some("yml") | Some("json") => {
            canonicalize_structured(&lf).unwrap_or_else(|| lf.into_bytes())
        }
        _ => lf.into_bytes(),
    }
}

fn canonicalize_structured(text: &str) -> Option<Vec<u8>> {
    let value: serde_yaml::Value = serde_yaml::from_str(text).ok()?;
    let sorted = sort_yaml(value);
    serde_yaml::to_string(&sorted).ok().map(String::into_bytes)
}

fn sort_yaml(value: serde_yaml::Value) -> serde_yaml::Value {
    match value {
        serde_yaml::Value::Mapping(map) => {
            let mut items: Vec<(serde_yaml::Value, serde_yaml::Value)> = map
                .into_iter()
                .map(|(k, v)| (sort_yaml(k), sort_yaml(v)))
                .collect();
            items.sort_by(|(a, _), (b, _)| yaml_key_cmp(a, b));
            let mut out = serde_yaml::Mapping::new();
            for (k, v) in items {
                out.insert(k, v);
            }
            serde_yaml::Value::Mapping(out)
        }
        serde_yaml::Value::Sequence(seq) => {
            serde_yaml::Value::Sequence(seq.into_iter().map(sort_yaml).collect())
        }
        serde_yaml::Value::Tagged(mut tagged) => {
            tagged.value = sort_yaml(tagged.value);
            serde_yaml::Value::Tagged(tagged)
        }
        other => other,
    }
}

fn yaml_key_cmp(a: &serde_yaml::Value, b: &serde_yaml::Value) -> std::cmp::Ordering {
    match (a, b) {
        (serde_yaml::Value::String(x), serde_yaml::Value::String(y)) => x.cmp(y),
        _ => {
            let sa = serde_yaml::to_string(a).unwrap_or_default();
            let sb = serde_yaml::to_string(b).unwrap_or_default();
            sa.cmp(&sb)
        }
    }
}

fn walk_err(err: walkdir::Error) -> StateError {
    match err.into_io_error() {
        Some(io) => StateError::Io(io),
        None => StateError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "walkdir failed",
        )),
    }
}
