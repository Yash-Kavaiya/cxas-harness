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
use std::fmt;
use std::sync::{Arc, Mutex, OnceLock};

/// Resource name of a CES agent snapshot created during hillclimb.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SnapshotName(pub String);

impl fmt::Display for SnapshotName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Blocking + async delete surface used by [`SnapshotGuard`].
#[allow(async_fn_in_trait)]
pub trait SnapshotApi {
    fn delete_snapshot_blocking(&self, name: &SnapshotName);
    async fn delete_snapshot(&self, name: &SnapshotName) -> Result<(), LifeError>;
}

/// Process-level registry of deletes that failed (or panicked) inside `Drop`.
fn failed_deletes() -> &'static Mutex<Vec<(SnapshotName, String)>> {
    static FAILED: OnceLock<Mutex<Vec<(SnapshotName, String)>>> = OnceLock::new();
    FAILED.get_or_init(|| Mutex::new(Vec::new()))
}

fn register_failed_delete(name: SnapshotName, msg: String) {
    tracing::error!(snapshot = %name, error = %msg, "snapshot delete failed in Drop");
    if let Ok(mut guard) = failed_deletes().lock() {
        guard.push((name, msg));
    }
}

/// Drain snapshots whose `Drop` delete failed. `HillclimbRun` inspects this after a loop.
pub fn take_failed_deletes() -> Vec<(SnapshotName, String)> {
    failed_deletes()
        .lock()
        .map(|mut guard| std::mem::take(&mut *guard))
        .unwrap_or_default()
}

/// Supervisor-drained names of snapshots whose owning task was dropped.
#[derive(Clone, Default)]
pub struct CleanupQueue {
    inner: Arc<Mutex<Vec<SnapshotName>>>,
}

impl CleanupQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&self, name: SnapshotName) {
        match self.inner.lock() {
            Ok(mut guard) => guard.push(name),
            Err(_) => register_failed_delete(name, "cleanup queue lock poisoned".into()),
        }
    }

    pub fn drain(&self) -> Vec<SnapshotName> {
        match self.inner.lock() {
            Ok(mut guard) => guard.drain(..).collect(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }
}

/// RAII guard: deletes the snapshot on drop unless [`SnapshotGuard::persist`] was called.
pub struct SnapshotGuard<T: SnapshotApi> {
    api: T,
    snapshot: SnapshotName,
    dismissed: bool,
    queue: Option<CleanupQueue>,
}

impl<T: SnapshotApi> SnapshotGuard<T> {
    pub fn new(api: T, snapshot: SnapshotName) -> Self {
        Self {
            api,
            snapshot,
            dismissed: false,
            queue: None,
        }
    }

    pub fn with_queue(api: T, snapshot: SnapshotName, queue: CleanupQueue) -> Self {
        Self {
            api,
            snapshot,
            dismissed: false,
            queue: Some(queue),
        }
    }

    /// Keep the snapshot. This is the only way a drop will skip delete.
    pub fn persist(mut self) {
        self.dismissed = true;
    }
}

impl<T: SnapshotApi> Drop for SnapshotGuard<T> {
    fn drop(&mut self) {
        if self.dismissed {
            return;
        }
        if let Some(queue) = &self.queue {
            queue.push(self.snapshot.clone());
        }
        // Drop must never panic, including when the blocking delete panics.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.api.delete_snapshot_blocking(&self.snapshot);
        }));
        if result.is_err() {
            register_failed_delete(
                self.snapshot.clone(),
                "delete_snapshot_blocking panicked".into(),
            );
        }
    }
}
