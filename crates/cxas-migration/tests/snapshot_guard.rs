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

use cxas_migration::{CleanupQueue, HillclimbRun, SnapshotApi, SnapshotGuard, SnapshotName};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct Rec {
    deleted: Arc<Mutex<Vec<String>>>,
}

impl SnapshotApi for Rec {
    fn delete_snapshot_blocking(&self, name: &SnapshotName) {
        self.deleted.lock().unwrap().push(name.0.clone());
    }
    async fn delete_snapshot(&self, name: &SnapshotName) -> Result<(), cxas_migration::LifeError> {
        self.delete_snapshot_blocking(name);
        Ok(())
    }
}

#[test]
fn drop_without_persist_deletes() {
    let api = Rec::default();
    {
        let _g = SnapshotGuard::new(api.clone(), SnapshotName("snap-1".into()));
    }
    assert_eq!(*api.deleted.lock().unwrap(), vec!["snap-1".to_string()]);
}

#[test]
fn persist_skips_delete() {
    let api = Rec::default();
    {
        let g = SnapshotGuard::new(api.clone(), SnapshotName("keep".into()));
        g.persist();
    }
    assert!(api.deleted.lock().unwrap().is_empty());
}

#[test]
fn panic_still_deletes() {
    let api = Rec::default();
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _g = SnapshotGuard::new(api.clone(), SnapshotName("boom".into()));
        panic!("hillclimb failed");
    }));
    assert!(caught.is_err());
    assert_eq!(*api.deleted.lock().unwrap(), vec!["boom".to_string()]);
}

#[tokio::test]
async fn aborted_task_enqueues_cleanup() {
    let q = CleanupQueue::new();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let handle = tokio::spawn({
        let q = q.clone();
        async move {
            let _g = SnapshotGuard::with_queue(Rec::default(), SnapshotName("aborted".into()), q);
            let _ = ready_tx.send(());
            std::future::pending::<()>().await;
        }
    });
    ready_rx.await.unwrap();
    handle.abort();
    let _ = handle.await;
    assert!(q.drain().iter().any(|n| n.0 == "aborted"));
}

#[tokio::test]
async fn iterate_deletes_losers_keeps_winner() {
    let api = Rec::default();
    let run = HillclimbRun {
        api: api.clone(),
        keep_winner: true,
    };
    let kept = run
        .iterate_named(&["a", "b", "c"], 2 /* winner index */)
        .await
        .unwrap();
    assert_eq!(kept, vec![SnapshotName("c".into())]);
    let mut deleted = api.deleted.lock().unwrap().clone();
    deleted.sort();
    assert_eq!(deleted, vec!["a".to_string(), "b".to_string()]);
}
