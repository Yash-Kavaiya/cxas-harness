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
use crate::snapshot::{take_failed_deletes, SnapshotApi, SnapshotGuard, SnapshotName};
use cxas_core::AppName;

/// Hillclimb loop that wraps every created snapshot in a [`SnapshotGuard`].
pub struct HillclimbRun<T: SnapshotApi> {
    pub api: T,
    pub keep_winner: bool,
}

impl<T: SnapshotApi + Clone> HillclimbRun<T> {
    /// Create `n` named snapshots under `parent`. If `keep_winner`, persist the last candidate.
    pub async fn iterate(
        &self,
        parent: &AppName,
        n: usize,
    ) -> Result<Vec<SnapshotName>, LifeError> {
        let names: Vec<String> = (0..n)
            .map(|i| format!("{}-candidate-{i}", parent.as_str()))
            .collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let winner_idx = n.saturating_sub(1);
        self.iterate_named(&refs, winner_idx).await
    }

    /// Wrap each name in a guard and `persist()` only `names[winner_idx]` when `keep_winner`.
    pub async fn iterate_named(
        &self,
        names: &[&str],
        winner_idx: usize,
    ) -> Result<Vec<SnapshotName>, LifeError> {
        let _ = take_failed_deletes();
        let mut kept = Vec::new();
        for (i, name) in names.iter().enumerate() {
            let snapshot = SnapshotName((*name).to_string());
            let guard = SnapshotGuard::new(self.api.clone(), snapshot.clone());
            if self.keep_winner && i == winner_idx {
                guard.persist();
                kept.push(snapshot);
            }
        }
        if let Some((name, msg)) = take_failed_deletes().into_iter().next() {
            return Err(LifeError::DeleteFailed(name, msg));
        }
        Ok(kept)
    }
}
