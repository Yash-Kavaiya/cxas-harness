# Phase 1 — Workspace Architecture Design

**Date:** 2026-08-15
**Status:** Approved from the product briefs (retired 2026-08-16; requirements restated here)
**Product:** `cxas-harness`
**Phase:** 1 of 5 — crate architecture
**Depends on:** `docs/superpowers/specs/2026-08-15-parity-contract-design.md`

## Purpose

Stand up the `cxas-harness` Cargo workspace so each Python module maps to an independently compilable Rust crate, with optional integrations behind feature flags. This phase ships the workspace skeleton, the `cxas-core` resource clients with a mandatory `Location`, the `cxas-state` content-addressed hasher, and the feature-flagged `cxas-utils` adapters.

This cycle is independently shippable: `cargo test --workspace` passes with no CES network, every crate compiles with default features, and `cxas-utils` default features do **not** pull Sheets, BigQuery, or audio crates.

**Issue-driven quality bar:** Phase 1 closes the packaging-bloat limitation class and provides the primitives that later phases use to close **#401** (hardcoded `"global"` `vertex_location`), **#270** (unified workspace resolution and cascading profiles), **#131** (`cxas diff` / `cxas state` drift primitives), **#263** (dedicated Evaluation `RunSession` quota — client surface only), **#403** (deployment channel settings API), **#252** (versioned pull primitive on `Apps`), and **#256** (boolean values in environment-template resolution). `cxas-scrapi` parity here means each `cxas-core` resource type listed in the Phase 0 manifest has a Rust struct whose constructor cannot omit `location`.

## Architecture

One Cargo workspace at the repository root. Crates communicate only through public types; no crate reaches into another's `mod` internals.

```
cxas-harness/
  Cargo.toml                 (workspace)
  parity/cxas-scrapi-parity.yaml
  proto/ces/
  crates/
    cxas-proto/              (Phase 0)
    cxas-parity/             (Phase 0)
    cxas-core/               (this phase: resource clients)
    cxas-utils/              (this phase: pagination + flagged adapters)
    cxas-state/              (this phase: hashing + profile resolution)
    cxas-evals/              (stub + quota client type; behavior in Phase 2)
    cxas-migration/          (stub; behavior in Phase 4)
    cxas-cli/                (stub binary; behavior in Phase 5)
    cxas-lint/               (stub; behavior in Phase 3)
```

Dependency direction (the Dependency Rule):

```
cxas-cli --> cxas-core, cxas-evals, cxas-migration, cxas-lint, cxas-state, cxas-utils
cxas-evals --> cxas-core, cxas-state, cxas-utils, cxas-proto
cxas-migration --> cxas-core, cxas-state, cxas-utils
cxas-lint --> cxas-state, cxas-utils
cxas-core --> cxas-proto, cxas-utils (default features only)
cxas-state --> (no CES deps)
cxas-utils --> optional google APIs behind features
cxas-parity --> (no CES deps)
```

`cxas-core` never depends on `cxas-evals` or `cxas-cli`. `location` is a `Location` newtype constructed only from a non-empty, non-defaulted string. There is no `Location::global()` constructor and no `Default` impl.

## Components

### 1. Workspace root `Cargo.toml`

```toml
[workspace]
resolver = "2"
members = [
  "crates/cxas-proto",
  "crates/cxas-parity",
  "crates/cxas-core",
  "crates/cxas-utils",
  "crates/cxas-state",
  "crates/cxas-evals",
  "crates/cxas-migration",
  "crates/cxas-cli",
  "crates/cxas-lint",
]
```

Shared workspace dependencies: `tokio`, `tonic`, `prost`, `serde`, `serde_json`, `serde_yaml`, `thiserror`, `async-trait`, `sha2`, `walkdir`, `clap`, `reqwest`. No `pandas`/`scikit-learn`/`pydub`/`gspread`/`InquirerPy` equivalents in default features.

### 2. `cxas-core` — resource clients

One module per Python resource type. Shared constructor:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Location(String);

impl Location {
    pub fn new(raw: impl Into<String>) -> Result<Self, CoreError>;
    pub fn as_str(&self) -> &str;
}

pub struct ClientConfig {
    pub project_id: String,
    pub location: Location,
    pub credentials: Credentials,
}

pub enum Credentials {
    ApplicationDefault,
    ServiceAccountPath(PathBuf),
    OauthToken(String),
}
```

`Location::new` returns `Err(CoreError::LocationRequired)` on empty/whitespace and `Err(CoreError::LocationHardcodedGlobalForbidden)` if a caller passes the sentinel `"__default_global__"` used only in tests to prove the Python bug cannot be reintroduced. Passing the literal `"global"` is **allowed** (some CES resources are genuinely global) but must be an explicit argument — never a field default, never `unwrap_or("global")`, never a `Default` impl.

Each client:

```rust
pub struct Apps { config: ClientConfig, transport: Arc<dyn CesTransport> }
impl Apps {
    pub fn new(config: ClientConfig) -> Self;
    pub async fn list_apps(&self) -> Result<Vec<App>, CoreError>;
    pub async fn get_app(&self, name: &AppName) -> Result<App, CoreError>;
    pub async fn get_app_by_display_name(&self, display: &str) -> Result<Option<App>, CoreError>;
    pub async fn create_app(&self, req: CreateAppRequest) -> Result<App, CoreError>;
    pub async fn delete_app(&self, name: &AppName, force: bool) -> Result<(), CoreError>;
    pub async fn export_app(&self, name: &AppName) -> Result<ExportHandle, CoreError>;
    pub async fn import_app(&self, name: &AppName, bytes: Bytes, strategy: ConflictStrategy) -> Result<App, CoreError>;
    pub async fn import_as_new_app(&self, display_name: &str, bytes: Bytes) -> Result<App, CoreError>;
    pub async fn export_app_version(&self, name: &AppName, version_id: &str) -> Result<ExportHandle, CoreError>; // #252
}
```

`CesTransport` is a trait so unit tests inject a mock. Default impl wraps `tonic` CES stubs from `cxas-proto`. Export uses **chunked / streamed** bytes (`ExportHandle` is an async stream of `Bytes`) so payloads ≥ 4 MB succeed (closes the size-limit class that is #298; the streaming implementation and its test live in this crate, the pull CLI flag in Phase 5).

Sibling clients with the same `ClientConfig` pattern: `Agents`, `Tools`, `Guardrails`, `Deployments`, `Sessions`, `Evaluations`, `Callbacks`, `Changelogs`, `ConversationHistory`, `Variables`, `Versions`.

`Deployments` includes channel settings (#403):

```rust
pub struct ChannelSettings {
    pub noise_cancellation: Option<bool>,
    pub noise_suppression_level: Option<u32>,
}
impl Deployments {
    pub async fn update_channel_settings(
        &self,
        deployment: &DeploymentName,
        settings: ChannelSettings,
    ) -> Result<Deployment, CoreError>;
}
```

`Evaluations` constructor takes `ClientConfig` (hence `location`) and a `QuotaKind`:

```rust
pub enum QuotaKind {
    RunSession,
    EvaluationRunSession, // #263 — default for evals
}
```

`SimulationEvals` (Phase 2) must construct its CES session client with `QuotaKind::EvaluationRunSession`. Phase 1 ships the enum and refuses to compile a `Sessions::new` call that does not receive a `QuotaKind` when the `eval-quota` feature of `cxas-core` is on (it is on by default).

### 3. `cxas-utils`

Default features: pagination (`Page<T>`), proto JSON flatten, environment-template renderer.

```rust
pub struct Page<T> { pub items: Vec<T>, pub next_page_token: Option<String> }
pub async fn paginate<F, Fut, T>(mut fetch: F) -> Result<Vec<T>, UtilsError>
where F: FnMut(Option<String>) -> Fut, Fut: Future<Output = Result<Page<T>, UtilsError>>;
```

Feature flags (off by default):

| Feature | Compiles | Python analogue |
|---|---|---|
| `sheets` | Google Sheets adapter | `GoogleSheetsUtils` / `gspread` |
| `bigquery` | BigQuery adapter | `pandas-gbq` / `google-cloud-bigquery` |
| `audio` | WAV/PCM helpers | `pydub` |
| `graphviz` | DOT export | `graphviz` |

Default `cxas-cli` and `cxas-core` depend on `cxas-utils` **without** those features. A `cargo tree -p cxas-cli` test (Phase 5) fails if `gspread`-class deps appear.

Environment-template renderer (#256):

```rust
pub fn render_environment(template: &serde_json::Value, vars: &BTreeMap<String, TemplateValue>) -> Result<serde_json::Value, UtilsError>;

pub enum TemplateValue {
    String(String),
    Bool(bool),
    Number(serde_json::Number),
}
```

Boolean placeholders such as `"{{ENABLE_VOICE}}"` resolve to JSON `true`/`false`, not the strings `"true"`/`"false"`.

### 4. `cxas-state`

No Python equivalent. Content-addressed hashing of an on-disk app directory and of a remote app snapshot.

```rust
pub struct StateHash(pub [u8; 32]); // SHA-256
pub fn hash_app_dir(root: &Path) -> Result<StateHash, StateError>;
pub fn hash_bytes(bytes: &[u8]) -> StateHash;
pub fn diff_trees(local: &AppTree, remote: &AppTree) -> StateDiff;

pub struct StateDiff {
    pub added: Vec<PathBuf>,
    pub removed: Vec<PathBuf>,
    pub changed: Vec<PathBuf>,
}

pub struct WorkspaceProfile {
    pub name: String,
    pub project_id: String,
    pub location: Location,
    pub extends: Option<String>,
}
pub fn resolve_workspace(cwd: &Path) -> Result<ResolvedWorkspace, StateError>;
```

`resolve_workspace` walks `cwd` → parents for `cxas.workspace.yaml`, then applies cascading profiles (`extends`) so a project-level file can override user-level defaults (#270). The resolved `location` is a `Location`; missing location is `StateError::LocationRequired`, never `"global"`.

`hash_app_dir` canonicalizes: UTF-8 paths with `/`, LF newlines, sorted keys in YAML/JSON, exclusion of `.git/` and `target/`. Tool-deletion sync (Phase 4, #394) diffs local `tools/` against the remote tree via `diff_trees`.

### 5. Stub crates

`cxas-evals`, `cxas-migration`, `cxas-lint`, `cxas-cli` each contain `pub fn crate_name() -> &'static str` and a `Cargo.toml` with the dependency edges above so the workspace compiles. Their real APIs are specified in later phase specs.

## Data flow

**Resource call (happy path):**

1. Caller builds `Location::new("us-central1")?` and `ClientConfig`.
2. `Apps::new(config)` stores config; does not dial yet.
3. `list_apps` asks `CesTransport` for page 1; `cxas-utils::paginate` follows tokens until `next_page_token` is `None`.
4. Results map through `cxas-proto` types into `cxas-core` domain structs (`App`, `Agent`, …).

**Workspace resolution (#270):**

1. `resolve_workspace(cwd)` finds the nearest `cxas.workspace.yaml`.
2. If `profile.extends` is set, load the named parent profile from the same file or `~/.config/cxas/profiles.yaml`.
3. Child keys overlay parent keys. `location` in the child, if present, replaces the parent; if absent in both, error.

**State hash (#131):**

1. `hash_app_dir` walks files, canonicalizes, SHA-256s the concatenation of `path + NUL + content`.
2. `diff_trees` compares two `AppTree` maps (path → hash).

**Large export (#298 class):**

1. `export_app` returns `ExportHandle` implementing `Stream<Item = Result<Bytes, CoreError>>`.
2. Consumers write chunks to a sink. A 5 MiB fixture must succeed in tests.

## Error handling

```rust
#[derive(Debug, thiserror::Error)]
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
    #[error(transparent)]
    Proto(#[from] prost::DecodeError),
}
```

| Condition | Result |
|---|---|
| Missing location in constructor / workspace | `LocationRequired` — never substitute `"global"` (#401) |
| Transport gRPC `NOT_FOUND` | `CoreError::NotFound` |
| Transport other status | `CoreError::Transport` with status code + message |
| Export stream shorter than advertised | `TruncatedExport` |
| Profile cycle in `extends` | `StateError::ProfileCycle` |
| Boolean template given a string `"maybe"` | `UtilsError::InvalidBoolTemplate` |

Library crates return `Result`. They do not print, do not `std::process::exit`, and do not prompt.

## Testing

1. **Workspace compiles** — `cargo metadata` lists all nine members; `cargo test --workspace --offline` (after deps cached) is the Phase 1 gate.
2. **#401** — `Location::new("")` errs; a test that constructs `Apps` without a `Location` does not compile (`trybuild`). A mock transport records the `location` metadata sent on the wire; feeding `"europe-west1"` asserts that exact string, never `"global"`.
3. **#270** — fixture dirs with parent/child `cxas.workspace.yaml`; child overlays `project_id`, inherits `location`; missing location errors.
4. **#131** — two fixture app trees that differ by one tool file; `diff_trees` reports that path in `changed` or `removed`.
5. **#263** — `Evaluations::new` with default config has `quota_kind == EvaluationRunSession`; a unit test on the mock transport asserts the request header / resource prefix reserved for eval quota.
6. **#403** — `update_channel_settings` sends `noise_cancellation: true` on the mock.
7. **#252** — `export_app_version` includes the version id in the mock request path.
8. **#256** — template `{"voice": "{{FLAG}}"}` with `TemplateValue::Bool(true)` yields JSON boolean `true`.
9. **#298 class** — mock export stream of 5 MiB in 64 KiB chunks reassembles to the original bytes.
10. **Feature isolation** — `cargo tree -p cxas-utils --edges normal` without `--features` does not contain `google-sheets`, `bigquery`, `rodio`, or `pydub` analogues.
11. **Parity hook** — `cxas-core` test loads `cxas_parity::load_bundled()` and asserts every `rust_owner == "cxas-core"` type has a same-named Rust struct in the crate (string presence via `stringify!` table maintained in `crates/cxas-core/src/parity_table.rs`).

No live CES/GCP calls.

## Out of scope

- Bidi session state machine and audio scoring (Phase 2).
- Lint rule registry (Phase 3).
- Snapshot RAII and DFCX conversion (Phase 4).
- Real `cxas` CLI parsing, `cargo-dist` binaries, mdBook (Phase 5).
- Executing Gauntlet Loop critics.
