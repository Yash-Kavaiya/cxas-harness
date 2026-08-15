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

use crate::error::LifeError;
use cxas_state::{diff_trees, hash_app_dir, AppTree, StateError};
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

/// Report of local tool paths deleted or retained after a remote reconcile.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SyncReport {
    pub deleted_local: Vec<PathBuf>,
    pub kept: Vec<PathBuf>,
}

/// Removes local `tools/` entries that are absent from a just-pulled remote tree.
pub struct ToolSync;

impl Default for ToolSync {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolSync {
    pub fn new() -> Self {
        Self
    }

    /// Delete local tools present in `local_root` but missing from `remote`.
    ///
    /// Added remote tools are not invented here; the next pull writes them.
    pub async fn reconcile(
        &self,
        local_root: &Path,
        remote: &AppTree,
    ) -> Result<SyncReport, LifeError> {
        let local = hash_app_dir(local_root)?;
        let local_tools = restrict_tools(&local);
        let remote_tools = restrict_tools(remote);
        let diff = diff_trees(&local_tools, &remote_tools);

        let root_canon = local_root
            .canonicalize()
            .map_err(StateError::from)?;

        let mut deleted_local = Vec::new();
        let mut prune_dirs = BTreeSet::new();

        for path in &diff.removed {
            let abs = local_root.join(path);
            verify_under_root(&abs, &root_canon)?;
            if abs.is_dir() {
                std::fs::remove_dir_all(&abs).map_err(StateError::from)?;
            } else if abs.is_file() {
                std::fs::remove_file(&abs).map_err(StateError::from)?;
            }
            deleted_local.push(path.clone());
            if let Some(dir) = tool_dir(path) {
                prune_dirs.insert(dir);
            }
        }

        for dir in prune_dirs {
            let abs = local_root.join(&dir);
            if !abs.is_dir() {
                continue;
            }
            verify_under_root(&abs, &root_canon)?;
            let empty = std::fs::read_dir(&abs)
                .map_err(StateError::from)?
                .next()
                .is_none();
            if empty {
                std::fs::remove_dir(&abs).map_err(StateError::from)?;
            }
        }

        let kept = local_tools
            .files
            .keys()
            .filter(|p| !deleted_local.iter().any(|d| d == *p))
            .cloned()
            .collect();

        Ok(SyncReport {
            deleted_local,
            kept,
        })
    }
}

fn restrict_tools(tree: &AppTree) -> AppTree {
    AppTree {
        files: tree
            .files
            .iter()
            .filter(|(path, _)| is_tools_path(path))
            .map(|(path, hash)| (path.clone(), *hash))
            .collect(),
        root_hash: tree.root_hash,
    }
}

fn is_tools_path(path: &Path) -> bool {
    matches!(
        path.components().next(),
        Some(Component::Normal(name)) if name == "tools"
    )
}

fn tool_dir(path: &Path) -> Option<PathBuf> {
    let mut comps = path.components();
    let first = comps.next()?;
    let second = comps.next()?;
    if first.as_os_str() == "tools" {
        Some(PathBuf::from("tools").join(second.as_os_str()))
    } else {
        None
    }
}

fn verify_under_root(path: &Path, root_canon: &Path) -> Result<(), LifeError> {
    let canon = match path.canonicalize() {
        Ok(canon) => canon,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(LifeError::State(StateError::PathEscape)),
    };
    if !contained_in(&canon, root_canon) {
        return Err(LifeError::State(StateError::PathEscape));
    }
    Ok(())
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
