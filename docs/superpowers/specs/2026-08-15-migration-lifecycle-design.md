# Phase 4 — Migration and Eval-Run Lifecycle Design

**Date:** 2026-08-15
**Status:** Approved from `PRD.md.txt` and `dev.md.txt`
**Product:** `cxas-harness`
**Phase:** 4 of 5 — migration and eval-run cleanup
**Depends on:** Phase 0 migration types, Phase 1 `cxas-core` / `cxas-state`, Phase 3 lint registry (optional post-deploy call)

## Purpose

Ship `cxas-migration` for Dialogflow CX → CXAS conversion (non-interactive by default) and resource-lifecycle guards so automated hillclimbing cannot leak agent snapshots. Tool deletions on the cloud are detected by hashing local vs remote trees.

**Issue-driven quality bar:** this phase closes **#168** (automated hillclimbing leaves behind entire agent snapshots) and **#394** (tool deletion in Cloud is not 100% reflected locally). `cxas-scrapi` parity means the Phase 0 migration types (`DFCXAgentExporter`, `ConversationalAgentsAPI`, `DFCXConversationRunner`, visualizers, `AIAugment`) exist as Rust structs, and `cxas migrate dfcx --run` has a non-interactive entry point that does not load a TUI.

## Architecture

Two subsystems in one crate family:

```
DFCX export (.zip or live agent)
        |
        v
cxas-migration pipeline (parse → IR bundle → CXAS app tree)
        |
        v
cxas-core import (location required)
        |
        + optional lint (cxas-lint)

Hillclimb loop
        |
        v
SnapshotGuard (RAII) --Drop--> delete snapshot even on panic
        |
        v
cxas-state::diff_trees  --tools/-->  local delete of cloud-removed tools (#394)
```

The TUI that Python loads via lazy `InquirerPy` is a **feature flag** (`tui`) on `cxas-migration`. Default builds expose only `--run` / `--optimize --yes`. This is structural, not a lazy import comment.

## Components

### 1. `SnapshotGuard` (#168)

```rust
pub struct SnapshotGuard<T: SnapshotApi> {
    api: T,
    snapshot: SnapshotName,
    dismissed: bool,
}

pub trait SnapshotApi {
    fn delete_snapshot(&self, name: &SnapshotName) -> impl Future<Output = Result<(), LifeError>>;
}

impl<T: SnapshotApi> SnapshotGuard<T> {
    pub fn new(api: T, snapshot: SnapshotName) -> Self;
    pub fn persist(mut self) { self.dismissed = true; } // keep on success path that intends to keep it
}

impl<T: SnapshotApi> Drop for SnapshotGuard<T> {
    fn drop(&mut self) {
        if self.dismissed { return; }
        // spawn_blocking / try_send to a cleanup queue; see Error handling
        self.api.delete_snapshot_blocking(&self.snapshot);
    }
}
```

Normative rules:

- Every hillclimb iteration that creates a snapshot **must** wrap it in `SnapshotGuard` before any `?` or fallible work.
- `persist()` is the only way to keep a snapshot. The happy-path "best candidate" calls `persist()` on the winner and lets losers drop.
- `Drop` must run on panic and on `tokio` task cancellation. For async deletion, the guard pushes the name onto a `CleanupQueue` (`std::sync::mpsc` or `tokio::sync::mpsc`) that a supervisor drains. Tests assert the queue received the name when the future is dropped.
- A process-level `atexit` is **not** an acceptable substitute; RAII + queue is required.

`HillclimbRun`:

```rust
pub struct HillclimbRun<T: SnapshotApi> {
    api: T,
    keep_winner: bool,
}

impl<T: SnapshotApi> HillclimbRun<T> {
    pub async fn iterate(&self, parent: &AppName, n: usize) -> Result<Vec<SnapshotName>, LifeError>;
}
```

`iterate` creates `n` snapshots, each under a guard. On error mid-loop, already-created guards drop. On success, if `keep_winner` only the highest-scoring snapshot is `persist()`ed.

### 2. Tool-deletion sync (#394)

```rust
pub struct ToolSync<'a> {
    tools: &'a Tools,
    state: &'a dyn StateHasher,
}

impl<'a> ToolSync<'a> {
    pub async fn reconcile(&self, local_root: &Path, remote: &AppTree) -> Result<SyncReport, LifeError>;
}
```

Algorithm:

1. Build `local = hash_app_dir(local_root)` tree restricted to `tools/`.
2. Take `remote` tools tree (from a just-pulled export or `list_tools` mapped into paths `tools/{id}/`).
3. `diff_trees(local, remote)`.
4. For each path in `diff.removed` **on the remote side** (present locally, absent remotely): delete the local path.
5. For each path in `diff.added` on the remote side: do not invent files; the next `pull` writes them. `reconcile` only removes stale local tools.
6. `SyncReport { deleted_local: Vec<PathBuf>, kept: Vec<PathBuf> }`.

A unit test starts with local tools `a` and `b`, remote tree containing only `a`, and asserts `b` is removed from a temp dir.

### 3. DFCX → CXAS pipeline (parity types)

```rust
pub struct DFCXAgentExporter { config: ClientConfig }
impl DFCXAgentExporter {
    pub async fn export_zip(&self, source_agent: &str) -> Result<PathBuf, MigrateError>;
}

pub struct ConversationalAgentsAPI { config: ClientConfig }

pub struct IrBundle { /* serde-serializable intermediate representation */ }

pub struct MigrationPipeline {
    pub profile: Profile, // Standard | Direct | Custom
    pub yes: bool,        // non-interactive; default true in cxas-harness
}

impl MigrationPipeline {
    pub async fn run(&self, src: MigrationSource, target: MigrationTarget) -> Result<MigratedApp, MigrateError>;
}

pub enum MigrationSource { AgentId(String), Zip(PathBuf) }
pub struct MigrationTarget { pub project_id: String, pub location: Location, pub display_name: String }
```

Stages (Python `--optimize --stage` 1/2/3):

| Stage | Input | Output |
|---|---|---|
| 0 transpile | DFCX zip / agent | 1:1 CXAS app tree |
| 1 | IR bundle | de-duplicated agents |
| 2 | IR bundle | goldens + lint + optimization report |
| 3 | IR bundle | hub-and-spoke wiring |

`--no-consolidate` skips 1–3 (Python `--no-optimize` alias recorded as deprecated-but-accepted in the parity manifest extensions). Default profile is `Standard` (consolidate). `Direct` is 1:1.

Visualizers (`FlowTreeVisualizer`, `HighLevelGraphVisualizer`, `MainVisualizer`, `PlaybookTreeVisualizer`, `FlowDependencyResolver`) render DOT or HTML **only** when the `graphviz` feature of `cxas-utils` is enabled. Without the feature they return `MigrateError::FeatureDisabled("graphviz")`.

`AIAugment` calls Gemini through the same HTTP client shape as `LlmLintClient` (`llm` feature).

`DFCXConversationRunner` produces `ConversationTrace` / `ConversationTurn` values used to seed goldens.

### 4. Non-interactive default

`MigrationPipeline { yes: true }` is the default. The `tui` feature compiles `MigrationTui` (ratatui). Invoking TUI entry points without the feature is a compile error. Python's default interactive dashboard is an intentional incompatibility, recorded on the parity command `["migrate", "dfcx"]` as `defaults.yes: true`.

Required `--run` arguments (fail before any CES call): `source` (`AgentId` or `Zip`), `project_id`, `location` (`Location::new`, no global default), `display_name`. Missing any of these is `MigrateError::Usage`.

## Data flow

**Non-interactive migrate**

1. Parse `MigrationSource` + `MigrationTarget` (`Location` required).
2. Export or read zip → IR bundle (optionally persist `<target>_ir.json` if `persist_bundle`).
3. Run stages per profile.
4. Import via `cxas_core::Apps::import_as_new_app`.
5. Optional `cxas_lint::run_all` on the produced tree (skipped if `--no-lint`).

**Hillclimb (#168)**

1. For candidate in 1..=n: create snapshot → `let guard = SnapshotGuard::new(...)`.
2. Evaluate candidate (Phase 2 `SimulationEvals` or a mock in this phase's tests).
3. If this candidate is not the winner, drop `guard` (delete). If it is the winner, `guard.persist()`.
4. Panic or cancel in step 2: `Drop` deletes the snapshot / enqueues deletion.

**Tool sync (#394)**

1. After a cloud-side tool delete (simulated in tests by shrinking the remote `AppTree`).
2. `ToolSync::reconcile` removes the local tool directory.
3. A subsequent `hash_app_dir` no longer lists that tool.

## Error handling

```rust
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
```

| Condition | Behavior |
|---|---|
| Snapshot delete fails in `Drop` | log via `tracing::error!`; push onto a process `FAILED_DELETES` registry that `HillclimbRun` inspects after the loop (`LifeError::DeleteFailed` if any remain) |
| Task cancelled | guard drops; queue receives the name |
| `--run` missing `--target-name` | `MigrateError::Usage` before network |
| Location omitted | `CoreError::LocationRequired` — never `"global"` |
| Graphviz feature off | `FeatureDisabled` |
| Local tool path is a symlink outside the app root | `StateError::PathEscape`; do not delete |

`Drop` never panics.

## Testing

1. **#168 happy path** — mock `SnapshotApi` records deletes; run 3 iterations, keep winner; assert 2 delete calls and 1 persisted name.
2. **#168 panic path** — `catch_unwind` around a closure that creates a guard then panics; assert delete was invoked (or the cleanup queue contains the name).
3. **#168 cancel path** — spawn a task that creates a guard, then `abort()` the task; after `await` of the join, assert the queue recorded the snapshot.
4. **#394** — temp dir with `tools/alpha` and `tools/beta`; remote tree only `tools/alpha`; `reconcile` deletes `beta` and leaves `alpha`.
5. **#394 no escape** — local "tool" is a symlink to `/tmp/outside`; `reconcile` returns `PathEscape` and does not delete the target.
6. **Usage** — `MigrationPipeline::run` without `display_name` returns `Usage` without calling the transport.
7. **Parity types** — `DFCXAgentExporter`, `ConversationalAgentsAPI`, `DFCXConversationRunner`, `ConversationTrace`, `ConversationTurn`, visualizer structs exist and are exported from `cxas_migration`.
8. **Non-interactive default** — `MigrationPipeline::default().yes == true`.
9. **Location** — target with empty location fails `Location::new` before export.

No live DFCX or CES calls. No TUI tests in the default feature set.

## Out of scope

- Compiling the `tui` feature's ratatui widgets (specified as compile-gated; default-feature tests do not cover ratatui widgets).
- `cxas migrate dfcx` clap wiring (Phase 5).
- Publishing binaries (Phase 5).
- Closing CLI ergonomics issues (#55, #54, #46, #350).
