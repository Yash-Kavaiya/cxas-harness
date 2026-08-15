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

mod diff;
mod hash;
mod workspace;

pub use diff::{diff_trees, StateDiff};
pub use hash::{hash_app_dir, hash_bytes, AppTree, StateHash};
pub use workspace::{resolve_workspace, ResolvedWorkspace, WorkspaceProfile};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StateError {
    #[error("location is required and has no default")]
    LocationRequired,
    #[error("profile extends cycle")]
    ProfileCycle,
    #[error("path escapes workspace root")]
    PathEscape,
    #[error("cxas.workspace.yaml not found from {0}")]
    WorkspaceNotFound(std::path::PathBuf),
    #[error("profile not found: {0}")]
    ProfileNotFound(String),
    #[error("active profile is not set")]
    ActiveProfileMissing,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
}

pub fn crate_name() -> &'static str {
    "cxas-state"
}
