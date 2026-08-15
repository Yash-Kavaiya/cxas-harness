# Phase 1 Workspace Architecture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up the `cxas-harness` Cargo workspace with `Location`-mandatory `cxas-core` clients, feature-flagged `cxas-utils`, and `cxas-state` hashing/profile resolution.

**Architecture:** Nine workspace members; dependency arrows only downward. `Location` has no `Default`. Optional Sheets/BigQuery/audio compile only behind features. Tests inject `CesTransport` mocks — no live CES.

**Tech Stack:** Rust 2021, MSRV 1.80, `tokio`, `thiserror`, `serde`/`serde_json`/`serde_yaml`, `sha2`, `async-trait`, `trybuild`, `walkdir`.

**Spec:** `docs/superpowers/specs/2026-08-15-workspace-architecture-design.md`

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

- Modify: `Cargo.toml` (add members)
- Create: `crates/cxas-core/Cargo.toml`, `src/lib.rs`, `src/location.rs`, `src/config.rs`, `src/error.rs`, `src/transport.rs`, `src/apps.rs`, `src/evaluations.rs`, `src/deployments.rs`, `src/parity_table.rs`
- Create: `crates/cxas-core/tests/location.rs`, `tests/export_stream.rs`, `trybuild/fail_apps_without_location.rs`
- Create: `crates/cxas-utils/Cargo.toml`, `src/lib.rs`, `src/page.rs`, `src/template.rs`
- Create: `crates/cxas-utils/tests/template.rs`
- Create: `crates/cxas-state/Cargo.toml`, `src/lib.rs`, `src/hash.rs`, `src/diff.rs`, `src/workspace.rs`
- Create: `crates/cxas-state/tests/workspace.rs`, `tests/hash_diff.rs`
- Create: stub crates `cxas-evals`, `cxas-migration`, `cxas-lint`, `cxas-cli` each with `src/lib.rs` or `src/main.rs`

---

### Task 1: `Location` newtype and `ClientConfig` (#401)

**Files:**
- Create: `crates/cxas-core/Cargo.toml`
- Create: `crates/cxas-core/src/location.rs`
- Create: `crates/cxas-core/src/error.rs`
- Create: `crates/cxas-core/src/config.rs`
- Create: `crates/cxas-core/src/lib.rs`
- Test: `crates/cxas-core/tests/location.rs`
- Test: `crates/cxas-core/trybuild/fail_apps_without_location.rs`
- Test: `crates/cxas-core/tests/trybuild.rs`
- Modify: root `Cargo.toml` members

**Interfaces:**
- Consumes: nothing
- Produces: `Location::new(raw) -> Result<Location, CoreError>`, `ClientConfig { project_id, location, credentials }`, `CoreError::LocationRequired`, `CoreError::LocationHardcodedGlobalForbidden`

- [ ] **Step 1: Write the failing test**

```rust
// crates/cxas-core/tests/location.rs
use cxas_core::{ClientConfig, CoreError, Credentials, Location};

#[test]
fn empty_location_is_rejected() {
    let err = Location::new("  ").unwrap_err();
    assert!(matches!(err, CoreError::LocationRequired));
}

#[test]
fn implicit_global_sentinel_is_rejected() {
    let err = Location::new("__default_global__").unwrap_err();
    assert!(matches!(err, CoreError::LocationHardcodedGlobalForbidden));
}

#[test]
fn explicit_global_is_allowed() {
    let loc = Location::new("global").unwrap();
    assert_eq!(loc.as_str(), "global");
}

#[test]
fn client_config_stores_the_given_location() {
    let cfg = ClientConfig {
        project_id: "demo".into(),
        location: Location::new("europe-west1").unwrap(),
        credentials: Credentials::ApplicationDefault,
    };
    assert_eq!(cfg.location.as_str(), "europe-west1");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-core empty_location_is_rejected --offline`
Expected: FAIL with `could not find package cxas-core`

- [ ] **Step 3: Write minimal implementation**

```rust
// crates/cxas-core/src/location.rs
use crate::CoreError;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Location(String);

impl Location {
    pub fn new(raw: impl Into<String>) -> Result<Self, CoreError> {
        let s = raw.into();
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(CoreError::LocationRequired);
        }
        if trimmed == "__default_global__" {
            return Err(CoreError::LocationHardcodedGlobalForbidden);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

```rust
// crates/cxas-core/src/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("location is required and has no default")]
    LocationRequired,
    #[error("refusing implicit global location sentinel")]
    LocationHardcodedGlobalForbidden,
    #[error("CES transport: {0}")]
    Transport(String),
    #[error("resource not found: {0}")]
    NotFound(String),
    #[error("invalid resource name: {0}")]
    InvalidName(String),
    #[error("export stream ended before content-length {expected} (got {got})")]
    TruncatedExport { expected: u64, got: u64 },
}
```

```rust
// crates/cxas-core/src/config.rs
use crate::Location;
use std::path::PathBuf;

pub enum Credentials {
    ApplicationDefault,
    ServiceAccountPath(PathBuf),
    OauthToken(String),
}

pub struct ClientConfig {
    pub project_id: String,
    pub location: Location,
    pub credentials: Credentials,
}
```

Export all from `lib.rs`. Add the package to the workspace. `Cargo.toml` depends on `thiserror = "2"`.

Add trybuild fixture that does `let _ = cxas_core::Apps::new();` once `Apps` exists in Task 2; for this task only the four unit tests are required. Create `tests/trybuild.rs` in Task 2.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-core --offline`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/cxas-core
git commit -m "feat(core): require explicit Location with no implicit global (#401)"
```

---

### Task 2: `Apps` + streamed export + mock transport (#298, #252)

**Files:**
- Create: `crates/cxas-core/src/transport.rs`
- Create: `crates/cxas-core/src/apps.rs`
- Test: `crates/cxas-core/tests/export_stream.rs`

**Interfaces:**
- Consumes: `ClientConfig`, `Location`
- Produces: `trait CesTransport`, `struct Apps`, `export_app`, `export_app_version`, `ExportHandle` (`Stream<Item = Result<Bytes, CoreError>>`)

- [ ] **Step 1: Write the failing test**

```rust
// crates/cxas-core/tests/export_stream.rs
use bytes::Bytes;
use cxas_core::{
    AppName, Apps, CesTransport, ClientConfig, Credentials, ExportRequest, Location,
};
use futures::StreamExt;
use std::sync::Arc;

struct FiveMegMock;

#[async_trait::async_trait]
impl CesTransport for FiveMegMock {
    async fn export_app(
        &self,
        req: ExportRequest,
    ) -> Result<cxas_core::ExportHandle, cxas_core::CoreError> {
        assert_eq!(req.location, "us-central1");
        assert_eq!(req.version_id.as_deref(), Some("v3"));
        let chunk = Bytes::from(vec![7u8; 64 * 1024]);
        let chunks = std::iter::repeat(chunk).take(80); // 5 MiB
        Ok(cxas_core::ExportHandle::from_iter(chunks))
    }
}

#[tokio::test]
async fn export_app_version_streams_five_megabytes() {
    let cfg = ClientConfig {
        project_id: "p".into(),
        location: Location::new("us-central1").unwrap(),
        credentials: Credentials::ApplicationDefault,
    };
    let apps = Apps::new(cfg, Arc::new(FiveMegMock));
    let handle = apps
        .export_app_version(&AppName::parse("projects/p/locations/us-central1/apps/a").unwrap(), "v3")
        .await
        .unwrap();
    let mut total = 0usize;
    futures::pin_mut!(handle);
    while let Some(part) = handle.next().await {
        total += part.unwrap().len();
    }
    assert_eq!(total, 5 * 1024 * 1024);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-core export_app_version_streams_five_megabytes --offline`
Expected: FAIL with `cannot find struct Apps` (or similar)

- [ ] **Step 3: Write minimal implementation**

Define `CesTransport` with `export_app(&self, ExportRequest { location: String, name: String, version_id: Option<String> })`. `Apps::new(config, transport)`. `export_app_version` copies `config.location.as_str()` into `ExportRequest.location` and sets `version_id`. `ExportHandle` wraps `Pin<Box<dyn Stream<Item = Result<Bytes, CoreError>> + Send>>` with `from_iter` helper mapping each `Bytes` to `Ok`.

Add deps: `bytes`, `async-trait`, `futures`, `tokio` (features `rt-multi-thread`, `macros`).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-core --offline`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cxas-core
git commit -m "feat(core): stream app exports and honor version-id (#298, #252)"
```

---

### Task 3: `QuotaKind`, channel settings, parity table (#263, #403)

**Files:**
- Create: `crates/cxas-core/src/evaluations.rs`
- Create: `crates/cxas-core/src/deployments.rs`
- Create: `crates/cxas-core/src/parity_table.rs`
- Test: `crates/cxas-core/tests/quota_and_channels.rs`

**Interfaces:**
- Consumes: `ClientConfig`, `CesTransport`
- Produces: `enum QuotaKind { RunSession, EvaluationRunSession }`, `Evaluations::new` defaults to `EvaluationRunSession`, `Deployments::update_channel_settings`

- [ ] **Step 1: Write the failing test**

```rust
use cxas_core::{
    ChannelSettings, ClientConfig, Credentials, Deployments, DeploymentName, Evaluations,
    Location, QuotaKind,
};
use std::sync::Arc;

#[test]
fn evaluations_default_to_evaluation_run_session_quota() {
    let cfg = ClientConfig {
        project_id: "p".into(),
        location: Location::new("us").unwrap(),
        credentials: Credentials::ApplicationDefault,
    };
    let ev = Evaluations::new(cfg, Arc::new(cxas_core::NoopTransport));
    assert_eq!(ev.quota_kind(), QuotaKind::EvaluationRunSession);
}

#[tokio::test]
async fn update_channel_settings_sends_noise_cancellation() {
    let transport = Arc::new(cxas_core::RecordingTransport::default());
    let cfg = ClientConfig {
        project_id: "p".into(),
        location: Location::new("us").unwrap(),
        credentials: Credentials::ApplicationDefault,
    };
    let deps = Deployments::new(cfg, transport.clone());
    deps.update_channel_settings(
        &DeploymentName::parse("projects/p/locations/us/apps/a/deployments/d").unwrap(),
        ChannelSettings {
            noise_cancellation: Some(true),
            noise_suppression_level: Some(2),
        },
    )
    .await
    .unwrap();
    let rec = transport.last_channel_settings().unwrap();
    assert_eq!(rec.noise_cancellation, Some(true));
    assert_eq!(rec.noise_suppression_level, Some(2));
}

#[test]
fn parity_table_covers_core_types() {
    let names = cxas_core::parity_table::CORE_PYTHON_CLASSES;
    assert!(names.contains(&"Apps"));
    assert!(names.contains(&"Evaluations"));
    assert!(names.contains(&"Deployments"));
    let manifest = cxas_parity::load_bundled().unwrap();
    for class in names {
        manifest.require_type(class).unwrap();
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-core evaluations_default_to_evaluation_run_session_quota --offline`
Expected: FAIL with `cannot find type Evaluations`

- [ ] **Step 3: Write minimal implementation**

`QuotaKind` enum. `Evaluations` stores `quota_kind: QuotaKind::EvaluationRunSession`. `NoopTransport` / `RecordingTransport` implement `CesTransport` (extend the trait with `update_channel_settings`). `parity_table::CORE_PYTHON_CLASSES` is a `&'static [&'static str]` listing every Phase 0 class owned by `cxas-core`. Depend on `cxas-parity`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-core --offline`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cxas-core
git commit -m "feat(core): eval quota kind and deployment channel settings (#263, #403)"
```

---

### Task 4: `cxas-utils` pagination and boolean templates (#256)

**Files:**
- Create: `crates/cxas-utils/Cargo.toml`
- Create: `crates/cxas-utils/src/lib.rs`
- Create: `crates/cxas-utils/src/page.rs`
- Create: `crates/cxas-utils/src/template.rs`
- Test: `crates/cxas-utils/tests/template.rs`

**Interfaces:**
- Consumes: nothing
- Produces: `Page<T>`, `paginate`, `TemplateValue::{String,Bool,Number}`, `render_environment`

- [ ] **Step 1: Write the failing test**

```rust
use cxas_utils::{render_environment, TemplateValue};
use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn boolean_placeholder_renders_as_json_bool() {
    let mut vars = BTreeMap::new();
    vars.insert("FLAG".into(), TemplateValue::Bool(true));
    let out = render_environment(&json!({"voice": "{{FLAG}}"}), &vars).unwrap();
    assert_eq!(out["voice"], json!(true));
    assert!(out["voice"].is_boolean());
}

#[test]
fn invalid_bool_string_is_error() {
    let mut vars = BTreeMap::new();
    vars.insert("FLAG".into(), TemplateValue::String("maybe".into()));
    let err = render_environment(&json!({"voice": "{{FLAG|bool}}"}), &vars).unwrap_err();
    assert!(matches!(err, cxas_utils::UtilsError::InvalidBoolTemplate));
}

#[tokio::test]
async fn paginate_follows_tokens() {
    use cxas_utils::{paginate, Page};
    let mut calls = 0u8;
    let items = paginate(|token| {
        calls += 1;
        let token = token.cloned();
        async move {
            match token.as_deref() {
                None => Ok(Page {
                    items: vec![1, 2],
                    next_page_token: Some("n".into()),
                }),
                Some("n") => Ok(Page {
                    items: vec![3],
                    next_page_token: None,
                }),
                _ => unreachable!(),
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(items, vec![1, 2, 3]);
    assert_eq!(calls, 2);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-utils boolean_placeholder_renders_as_json_bool --offline`
Expected: FAIL with `could not find package cxas-utils`

- [ ] **Step 3: Write minimal implementation**

`render_environment` walks JSON strings; if a string is exactly `{{NAME}}` and `vars[NAME]` is `Bool`, replace the value with JSON bool. `{{NAME|bool}}` parses a string var as bool (`true`/`false` only) or returns `InvalidBoolTemplate`. `paginate` loops until `next_page_token` is `None`. Default features empty; declare `[features] sheets = []`, `bigquery = []`, `audio = []`, `graphviz = []` as placeholders that compile nothing yet.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-utils --offline`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/cxas-utils
git commit -m "feat(utils): paginate and render boolean environment templates (#256)"
```

---

### Task 5: `cxas-state` hash, diff, cascading profiles (#131, #270)

**Files:**
- Create: `crates/cxas-state/Cargo.toml`
- Create: `crates/cxas-state/src/lib.rs`
- Create: `crates/cxas-state/src/hash.rs`
- Create: `crates/cxas-state/src/diff.rs`
- Create: `crates/cxas-state/src/workspace.rs`
- Test: `crates/cxas-state/tests/hash_diff.rs`
- Test: `crates/cxas-state/tests/workspace.rs`

**Interfaces:**
- Consumes: `cxas_core::Location`
- Produces: `StateHash`, `hash_app_dir`, `diff_trees`, `resolve_workspace`, `StateError::LocationRequired`, `StateError::ProfileCycle`

- [ ] **Step 1: Write the failing test**

```rust
// crates/cxas-state/tests/hash_diff.rs
use cxas_state::{diff_trees, hash_app_dir, AppTree};
use std::fs;
use std::path::PathBuf;

fn write_tree(root: &std::path::Path, files: &[(&str, &str)]) {
    for (rel, body) in files {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }
}

#[test]
fn diff_reports_removed_tool() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    write_tree(&a, &[("tools/alpha/tool.yaml", "x: 1"), ("tools/beta/tool.yaml", "y: 2")]);
    write_tree(&b, &[("tools/alpha/tool.yaml", "x: 1")]);
    let left = hash_app_dir(&a).unwrap();
    let right = hash_app_dir(&b).unwrap();
    let diff = diff_trees(&left, &right);
    assert!(diff.removed.iter().any(|p| p == &PathBuf::from("tools/beta/tool.yaml")));
}

#[test]
fn identical_trees_have_equal_hashes() {
    let tmp = tempfile::tempdir().unwrap();
    write_tree(tmp.path(), &[("app.yaml", "display_name: d\n")]);
    let once = hash_app_dir(tmp.path()).unwrap();
    let twice = hash_app_dir(tmp.path()).unwrap();
    assert_eq!(once.root_hash, twice.root_hash);
}
```

```rust
// crates/cxas-state/tests/workspace.rs
use cxas_state::{resolve_workspace, StateError};
use std::fs;

#[test]
fn child_profile_overlays_parent_and_keeps_location() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("cxas.workspace.yaml"),
        r#"
profiles:
  base:
    project_id: parent-proj
    location: us-central1
  child:
    extends: base
    project_id: child-proj
active: child
"#,
    )
    .unwrap();
    let ws = resolve_workspace(tmp.path()).unwrap();
    assert_eq!(ws.project_id, "child-proj");
    assert_eq!(ws.location.as_str(), "us-central1");
}

#[test]
fn missing_location_is_error_not_global() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("cxas.workspace.yaml"),
        "profiles:\n  x:\n    project_id: p\nactive: x\n",
    )
    .unwrap();
    let err = resolve_workspace(tmp.path()).unwrap_err();
    assert!(matches!(err, StateError::LocationRequired));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-state diff_reports_removed_tool --offline`
Expected: FAIL with `could not find package cxas-state`

- [ ] **Step 3: Write minimal implementation**

`hash_app_dir` walks files (skip `.git`, `target`), normalizes paths to `/`, SHA-256s `path + 0x00 + bytes`, stores per-path hashes in `AppTree { files: BTreeMap<PathBuf, StateHash>, root_hash: StateHash }`. `diff_trees(local, remote)`: paths only in local → `removed` (from remote's perspective the extra local files are "removed on remote"); paths only in remote → `added`; both but hash differs → `changed`. `resolve_workspace` reads YAML, follows `extends` with a seen-set (cycle → `ProfileCycle`), requires `location` via `Location::new`.

Deps: `sha2`, `walkdir`, `serde`, `serde_yaml`, `thiserror`, `cxas-core`, `tempfile` (dev).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-state --offline`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/cxas-state
git commit -m "feat(state): content-addressed hash, diff, cascading profiles (#131, #270)"
```

---

### Task 6: Stub remaining crates so the workspace compiles

**Files:**
- Create: `crates/cxas-evals/Cargo.toml`, `src/lib.rs`
- Create: `crates/cxas-migration/Cargo.toml`, `src/lib.rs`
- Create: `crates/cxas-lint/Cargo.toml`, `src/lib.rs`
- Create: `crates/cxas-cli/Cargo.toml`, `src/main.rs`
- Modify: root `Cargo.toml` members

**Interfaces:**
- Consumes: declared deps only (`cxas-evals` → `cxas-core`, `cxas-state`, `cxas-utils`, `cxas-proto`)
- Produces: `pub fn crate_name() -> &'static str` in each lib; `cxas-cli` `fn main()` prints `cxas-harness` and exits 0

- [ ] **Step 1: Write the failing test**

```rust
// crates/cxas-evals/tests/stub.rs
#[test]
fn crate_name_is_evals() {
    assert_eq!(cxas_evals::crate_name(), "cxas-evals");
}
```

Repeat the same assertion pattern in `crates/cxas-migration/tests/stub.rs` (`"cxas-migration"`) and `crates/cxas-lint/tests/stub.rs` (`"cxas-lint"`).

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-evals crate_name_is_evals --offline`
Expected: FAIL with `could not find package cxas-evals`

- [ ] **Step 3: Write minimal implementation**

Each lib: `pub fn crate_name() -> &'static str { "cxas-evals" }` (and the matching string for the others). `cxas-cli` is a bin with `fn main() { println!("cxas-harness"); }`. Wire workspace members and the spec's dependency edges. Do not enable `sheets`/`bigquery`/`audio` features.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --workspace --offline`
Expected: PASS (all Phase 0 + Phase 1 tests)

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/cxas-evals crates/cxas-migration crates/cxas-lint crates/cxas-cli
git commit -m "chore(workspace): add evals, migration, lint, and cli crate stubs"
```
