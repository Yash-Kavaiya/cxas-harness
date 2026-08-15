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

use crate::AppTree;
use std::path::PathBuf;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StateDiff {
    pub added: Vec<PathBuf>,
    pub removed: Vec<PathBuf>,
    pub changed: Vec<PathBuf>,
}

impl StateDiff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }
}

/// Compare `local` to `remote`.
///
/// Paths only in `local` are `removed` (absent on remote). Paths only in
/// `remote` are `added`. Shared paths with different hashes are `changed`.
pub fn diff_trees(local: &AppTree, remote: &AppTree) -> StateDiff {
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();

    for (path, hash) in &local.files {
        match remote.files.get(path) {
            None => removed.push(path.clone()),
            Some(other) if other != hash => changed.push(path.clone()),
            Some(_) => {}
        }
    }
    for path in remote.files.keys() {
        if !local.files.contains_key(path) {
            added.push(path.clone());
        }
    }

    StateDiff {
        added,
        removed,
        changed,
    }
}
