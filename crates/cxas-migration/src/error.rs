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

use crate::snapshot::SnapshotName;

#[derive(Debug, thiserror::Error)]
pub enum LifeError {
    #[error("snapshot delete failed for {0}: {1}")]
    DeleteFailed(SnapshotName, String),
    #[error("cleanup queue closed")]
    QueueClosed,
    #[error(transparent)]
    Core(#[from] cxas_core::CoreError),
    #[error(transparent)]
    State(#[from] cxas_state::StateError),
}

#[derive(Debug, thiserror::Error)]
pub enum MigrateError {
    #[error("usage: {0}")]
    Usage(&'static str),
    #[error("feature {0} is not enabled")]
    FeatureDisabled(&'static str),
    #[error("ir bundle invalid: {0}")]
    Ir(String),
    #[error(transparent)]
    Core(#[from] cxas_core::CoreError),
}
