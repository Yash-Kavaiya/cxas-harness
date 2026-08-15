# Phase 4 Migration and Lifecycle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement RAII snapshot cleanup for hillclimbing, local/remote tool-deletion sync via `cxas-state`, and a non-interactive DFCX → CXAS pipeline skeleton.

**Architecture:** `SnapshotGuard` deletes on `Drop` unless `persist()` is called. `ToolSync::reconcile` removes local tools absent from the remote tree. `MigrationPipeline` defaults to `yes: true` and requires `Location`.

**Tech Stack:** Rust 2021, `tokio`, `thiserror`, `serde`/`serde_yaml`, `cxas-core`, `cxas-state`, `tempfile` (dev).

**Spec:** `docs/superpowers/specs/2026-08-15-migration-lifecycle-design.md`

## Global Constraints

- Language: Rust 2021 edition, MSRV 1.80.
- Async runtime: `tokio` (full) only in crates that perform I/O; `cxas-parity` is sync.
- gRPC/protobuf: `tonic` + `prost` only; no Python protobuf stubs.
- `location` is never defaulted to `"global"`.
- Feature flags isolate optional integrations (Sheets, BigQuery, TUI, audio).
- Machine-first CLI: structured JSON, stable exit codes, non-interactive by default.
- Issue-driven quality bar: 25 cataloged `GoogleCloudPlatform/cxas-scrapi` issues each require a closing test before release candidate.
- Apache-2.0 license headers on every new Rust file.
- No Gauntlet Loop runtime; Superpowers spec→plan is the development process for this repository.

---

## File map

- Modify: `crates/cxas-migration/Cargo.toml`, `src/lib.rs`
- Create: `crates/cxas-migration/src/error.rs`
- Create: `crates/cxas-migration/src/snapshot.rs`
- Create: `crates/cxas-migration/src/hillclimb.rs`
- Create: `crates/cxas-migration/src/tool_sync.rs`
- Create: `crates/cxas-migration/src/pipeline.rs`
- Create: `crates/cxas-migration/src/dfcx.rs`
- Test: `crates/cxas-migration/tests/snapshot_guard.rs`
- Test: `crates/cxas-migration/tests/tool_sync.rs`
- Test: `crates/cxas-migration/tests/pipeline.rs`

---

### Task 1: `SnapshotGuard` deletes on drop, panic, and cancel (#168)

**Files:**
- Create: `crates/cxas-migration/src/error.rs`
- Create: `crates/cxas-migration/src/snapshot.rs`
- Create: `crates/cxas-migration/src/hillclimb.rs`
- Test: `crates/cxas-migration/tests/snapshot_guard.rs`

**Interfaces:**
- Consumes: `trait SnapshotApi { fn delete_snapshot_blocking(&self, name: &SnapshotName); async fn delete_snapshot(...); }`
- Produces: `SnapshotGuard::{new, persist}`, `Drop`, `CleanupQueue`, `HillclimbRun::iterate`

- [ ] **Step 1: Write the failing test**

```rust
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
    let handle = tokio::spawn({
        let q = q.clone();
        async move {
            let _g = SnapshotGuard::with_queue(Rec::default(), SnapshotName("aborted".into()), q);
            std::future::pending::<()>().await;
        }
    });
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
    let kept = run.iterate_named(&["a", "b", "c"], 2 /* winner index */).await.unwrap();
    assert_eq!(kept, vec![SnapshotName("c".into())]);
    let mut deleted = api.deleted.lock().unwrap().clone();
    deleted.sort();
    assert_eq!(deleted, vec!["a".to_string(), "b".to_string()]);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-migration drop_without_persist_deletes --offline`
Expected: FAIL with `cannot find struct SnapshotGuard`

- [ ] **Step 3: Write minimal implementation**

```rust
pub struct SnapshotName(pub String);

pub struct SnapshotGuard<T: SnapshotApi> {
    api: T,
    snapshot: SnapshotName,
    dismissed: bool,
    queue: Option<CleanupQueue>,
}

impl<T: SnapshotApi> Drop for SnapshotGuard<T> {
    fn drop(&mut self) {
        if self.dismissed {
            return;
        }
        if let Some(q) = &self.queue {
            q.push(self.snapshot.clone());
        }
        self.api.delete_snapshot_blocking(&self.snapshot);
    }
}

impl<T: SnapshotApi> SnapshotGuard<T> {
    pub fn persist(mut self) {
        self.dismissed = true;
    }
}
```

`CleanupQueue` is `Arc<Mutex<Vec<SnapshotName>>>` with `push`/`drain`. `HillclimbRun::iterate_named(names, winner_idx)` wraps each name in a guard and `persist()`s only the winner. `Drop` must not panic.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-migration --offline`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cxas-migration
git commit -m "feat(migration): RAII snapshot guards clean hillclimb leftovers (#168)"
```

---

### Task 2: `ToolSync::reconcile` reflects cloud deletions locally (#394)

**Files:**
- Create: `crates/cxas-migration/src/tool_sync.rs`
- Test: `crates/cxas-migration/tests/tool_sync.rs`

**Interfaces:**
- Consumes: `cxas_state::{hash_app_dir, diff_trees, AppTree}`, local `tools/` directory
- Produces: `ToolSync::reconcile`, `SyncReport { deleted_local, kept }`, `StateError::PathEscape` propagated as `LifeError`

- [ ] **Step 1: Write the failing test**

```rust
use cxas_state::{hash_app_dir, AppTree};
use cxas_migration::ToolSync;
use std::fs;
use std::path::PathBuf;

#[tokio::test]
async fn deletes_local_tool_missing_from_remote() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir_all(root.join("tools/alpha")).unwrap();
    fs::write(root.join("tools/alpha/tool.yaml"), "name: alpha\n").unwrap();
    fs::create_dir_all(root.join("tools/beta")).unwrap();
    fs::write(root.join("tools/beta/tool.yaml"), "name: beta\n").unwrap();

    let remote_dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(remote_dir.path().join("tools/alpha")).unwrap();
    fs::write(remote_dir.path().join("tools/alpha/tool.yaml"), "name: alpha\n").unwrap();
    let remote = hash_app_dir(remote_dir.path()).unwrap();

    let report = ToolSync::new().reconcile(root, &remote).await.unwrap();
    assert!(!root.join("tools/beta").exists());
    assert!(root.join("tools/alpha").exists());
    assert!(report.deleted_local.iter().any(|p| p == &PathBuf::from("tools/beta/tool.yaml")
        || p.starts_with("tools/beta")));
}

#[tokio::test]
async fn refuses_to_follow_symlink_outside_root() {
    let tmp = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret"), "nope").unwrap();
    fs::create_dir_all(tmp.path().join("tools")).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.path(), tmp.path().join("tools/evil")).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(outside.path(), tmp.path().join("tools/evil")).unwrap();

    let remote = AppTree::empty();
    let err = ToolSync::new().reconcile(tmp.path(), &remote).await.unwrap_err();
    assert!(matches!(err, cxas_migration::LifeError::State(cxas_state::StateError::PathEscape)));
    assert!(outside.path().join("secret").exists());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-migration deletes_local_tool_missing_from_remote --offline`
Expected: FAIL with `cannot find struct ToolSync`

- [ ] **Step 3: Write minimal implementation**

`ToolSync::reconcile`:
1. `local = hash_app_dir(local_root)?` (hash_app_dir must return `PathEscape` when a `tools/` entry canonicalizes outside `local_root` — add that check in `cxas-state` if missing, with a unit test in this crate covering the error mapping).
2. `diff = diff_trees(&local, remote)`.
3. For each `removed` path under `tools/`, `std::fs::remove_file` or `remove_dir_all` on the local path after verifying `canonicalize` stays under `local_root`.
4. Return `SyncReport`.

If `hash_app_dir` does not yet emit `PathEscape`, add to `crates/cxas-state/src/hash.rs`:

```rust
if path.canonicalize()?.starts_with(root.canonicalize()?) == false {
    return Err(StateError::PathEscape);
}
```

and a `StateError::PathEscape` variant.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-migration --offline`
Run: `cargo test -p cxas-state --offline`
Expected: PASS both

- [ ] **Step 5: Commit**

```bash
git add crates/cxas-migration crates/cxas-state
git commit -m "feat(migration): reconcile local tools against remote state hash (#394)"
```

---

### Task 3: Non-interactive migration pipeline and parity types

**Files:**
- Create: `crates/cxas-migration/src/pipeline.rs`
- Create: `crates/cxas-migration/src/dfcx.rs`
- Test: `crates/cxas-migration/tests/pipeline.rs`

**Interfaces:**
- Consumes: `cxas_core::{ClientConfig, Location}`
- Produces: `MigrationPipeline { profile, yes }` default `yes: true`, `run(src, target)`, `MigrationTarget.location: Location`, types `DFCXAgentExporter`, `ConversationalAgentsAPI`, `DFCXConversationRunner`, `ConversationTrace`, `ConversationTurn`, visualizer structs

- [ ] **Step 1: Write the failing test**

```rust
use cxas_core::Location;
use cxas_migration::{
    ConversationalAgentsAPI, DFCXAgentExporter, DFCXConversationRunner, MigrationPipeline,
    MigrationSource, MigrationTarget, Profile,
};

#[test]
fn default_pipeline_is_non_interactive() {
    let p = MigrationPipeline::default();
    assert!(p.yes);
    assert_eq!(p.profile, Profile::Standard);
}

#[tokio::test]
async fn run_without_display_name_is_usage_error() {
    let p = MigrationPipeline::default();
    let err = p
        .run(
            MigrationSource::Zip(std::path::PathBuf::from("agent.zip")),
            MigrationTarget {
                project_id: "p".into(),
                location: Location::new("us").unwrap(),
                display_name: String::new(),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, cxas_migration::MigrateError::Usage(_)));
}

#[test]
fn location_new_rejects_blank_before_export() {
    assert!(Location::new("").is_err());
}

#[test]
fn parity_types_are_exported() {
    let _ = std::any::type_name::<DFCXAgentExporter>();
    let _ = std::any::type_name::<ConversationalAgentsAPI>();
    let _ = std::any::type_name::<DFCXConversationRunner>();
    let _ = std::any::type_name::<cxas_migration::FlowTreeVisualizer>();
    let _ = std::any::type_name::<cxas_migration::HighLevelGraphVisualizer>();
    let _ = std::any::type_name::<cxas_migration::MainVisualizer>();
    let _ = std::any::type_name::<cxas_migration::PlaybookTreeVisualizer>();
    let _ = std::any::type_name::<cxas_migration::FlowDependencyResolver>();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-migration default_pipeline_is_non_interactive --offline`
Expected: FAIL with `cannot find struct MigrationPipeline`

- [ ] **Step 3: Write minimal implementation**

```rust
pub enum Profile { Standard, Direct, Custom }
pub enum MigrationSource { AgentId(String), Zip(PathBuf) }
pub struct MigrationTarget {
    pub project_id: String,
    pub location: Location,
    pub display_name: String,
}
pub struct MigrationPipeline {
    pub profile: Profile,
    pub yes: bool,
}
impl Default for MigrationPipeline {
    fn default() -> Self {
        Self { profile: Profile::Standard, yes: true }
    }
}
impl MigrationPipeline {
    pub async fn run(&self, src: MigrationSource, target: MigrationTarget) -> Result<MigratedApp, MigrateError> {
        if target.display_name.trim().is_empty() {
            return Err(MigrateError::Usage("target-name is required"));
        }
        let _ = src;
        Ok(MigratedApp { display_name: target.display_name })
    }
}
```

Empty pub structs for each parity type in `dfcx.rs`. Visualizer methods that need graphviz return `MigrateError::FeatureDisabled("graphviz")` when the feature is off:

```rust
impl FlowTreeVisualizer {
    pub fn render_dot(&self) -> Result<String, MigrateError> {
        Err(MigrateError::FeatureDisabled("graphviz"))
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-migration --offline`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cxas-migration
git commit -m "feat(migration): non-interactive DFCX pipeline skeleton with parity types"
```
