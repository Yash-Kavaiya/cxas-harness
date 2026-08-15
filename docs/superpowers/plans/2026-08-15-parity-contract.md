# Phase 0 Parity Contract Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Freeze the `cxas-scrapi` public surface as a machine-readable parity manifest and generate `cxas-proto` bindings that close GitHub issue #284 by typing unknown `EvaluationRunState` wire values.

**Architecture:** A sync `cxas-parity` crate loads a checked-in YAML contract; a `cxas-proto` crate compiles vendored CES protos with an `Unknown(i32)` enum wrapper. No CES network I/O.

**Tech Stack:** Rust 2021, MSRV 1.80, `serde`/`serde_yaml`/`thiserror`, `tonic`/`prost`/`tonic-build`, `trybuild` for the exhaustive-match compile-fail fixture.

**Spec:** `docs/superpowers/specs/2026-08-15-parity-contract-design.md`

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

- Create: `Cargo.toml` (workspace root)
- Create: `parity/cxas-scrapi-parity.yaml`
- Create: `crates/cxas-parity/Cargo.toml`
- Create: `crates/cxas-parity/src/lib.rs`
- Create: `crates/cxas-parity/src/error.rs`
- Create: `crates/cxas-parity/src/manifest.rs`
- Create: `crates/cxas-parity/tests/manifest_contract.rs`
- Create: `proto/ces/evaluation_run_state.proto` (minimal vendored enum used until full CES protos are copied)
- Create: `crates/cxas-proto/Cargo.toml`
- Create: `crates/cxas-proto/build.rs`
- Create: `crates/cxas-proto/src/lib.rs`
- Create: `crates/cxas-proto/src/evaluation_run_state.rs`
- Create: `crates/cxas-proto/tests/unknown_state.rs`
- Create: `crates/cxas-proto/trybuild/fail_missing_unknown.rs`
- Create: `crates/cxas-proto/tests/trybuild.rs`

---

### Task 1: Workspace + `cxas-parity` loader

**Files:**
- Create: `Cargo.toml`
- Create: `crates/cxas-parity/Cargo.toml`
- Create: `crates/cxas-parity/src/error.rs`
- Create: `crates/cxas-parity/src/manifest.rs`
- Create: `crates/cxas-parity/src/lib.rs`
- Create: `parity/cxas-scrapi-parity.yaml` (minimal valid file for this task)
- Test: `crates/cxas-parity/tests/manifest_contract.rs`

**Interfaces:**
- Consumes: nothing (first crate)
- Produces: `pub fn load_manifest(path: &Path) -> Result<ParityManifest, ParityError>`, `pub fn load_bundled() -> Result<ParityManifest, ParityError>`, `ParityManifest { version, source, modules, enums, cli, issue_gates }`

- [ ] **Step 1: Write the failing test**

```rust
// crates/cxas-parity/tests/manifest_contract.rs
use cxas_parity::{load_bundled, ParityError};

#[test]
fn bundled_manifest_loads_and_has_version_1() {
    let m = load_bundled().expect("bundled YAML must parse");
    assert_eq!(m.version, 1);
    assert_eq!(
        m.source.commit,
        "4f7b43ca6adda0acad95a7e3654eee4e2ed1438c"
    );
}

#[test]
fn missing_file_is_io_error() {
    let err = cxas_parity::load_manifest(std::path::Path::new(
        "this/path/does/not/exist.yaml",
    ))
    .unwrap_err();
    assert!(matches!(err, ParityError::Io(_)));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-parity bundled_manifest_loads_and_has_version_1 --offline`
Expected: FAIL with `error: could not find crate cxas_parity` or `could not find package cxas-parity`

- [ ] **Step 3: Write minimal implementation**

Root `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = ["crates/cxas-parity"]
```

`crates/cxas-parity/Cargo.toml`:

```toml
[package]
name = "cxas-parity"
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"
rust-version = "1.80"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
serde_json = "1"
thiserror = "2"
```

`crates/cxas-parity/src/error.rs`:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParityError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("yaml: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("schema: {0}")]
    Schema(String),
    #[error("duplicate: {0}")]
    Duplicate(String),
    #[error("missing: {0}")]
    Missing(String),
}
```

`crates/cxas-parity/src/manifest.rs`:

```rust
use crate::ParityError;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub repository: String,
    pub commit: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParityType {
    pub python_class: String,
    pub python_module: String,
    pub rust_type: String,
    pub methods: Vec<ParityMethod>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParityMethod {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParityModule {
    pub name: String,
    pub rust_owner: String,
    #[serde(default)]
    pub types: Vec<ParityType>,
    #[serde(default)]
    pub commands: Vec<ParityCommand>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParityCommand {
    pub argv: Vec<String>,
    pub python_handler: String,
    pub rust_owner: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParityEnum {
    pub python_name: String,
    pub proto_type: String,
    pub rust_type: String,
    pub rust_owner: String,
    pub unknown_policy: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IssueGate {
    pub id: u32,
    pub crate_name: String,
    pub test: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cli {
    pub binary: String,
    #[serde(default)]
    pub global_flags: Vec<String>,
    #[serde(default)]
    pub commands: Vec<ParityCommand>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParityManifest {
    pub version: u32,
    pub source: Source,
    pub modules: Vec<ParityModule>,
    pub enums: Vec<ParityEnum>,
    pub cli: Cli,
    pub issue_gates: Vec<IssueGate>,
}

const BUNDLED: &str = include_str!("../../../parity/cxas-scrapi-parity.yaml");

impl ParityManifest {
    pub fn require_type(&self, python_class: &str) -> Result<&ParityType, ParityError> {
        self.modules
            .iter()
            .flat_map(|m| m.types.iter())
            .find(|t| t.python_class == python_class)
            .ok_or_else(|| ParityError::Missing(python_class.into()))
    }

    pub fn require_command(&self, argv: &[&str]) -> Result<&ParityCommand, ParityError> {
        let wanted: Vec<String> = argv.iter().map(|s| (*s).to_string()).collect();
        self.cli
            .commands
            .iter()
            .find(|c| c.argv == wanted)
            .ok_or_else(|| ParityError::Missing(wanted.join(" ")))
    }

    pub fn types_for_crate(&self, rust_owner: &str) -> Vec<&ParityType> {
        self.modules
            .iter()
            .filter(|m| m.rust_owner == rust_owner)
            .flat_map(|m| m.types.iter())
            .collect()
    }

    pub fn commands_for_crate(&self, rust_owner: &str) -> Vec<&ParityCommand> {
        self.cli
            .commands
            .iter()
            .filter(|c| c.rust_owner == rust_owner)
            .collect()
    }

    pub fn issue_gates(&self) -> &[IssueGate] {
        &self.issue_gates
    }

    pub fn to_json(&self) -> Result<String, ParityError> {
        serde_json::to_string_pretty(self).map_err(|e| ParityError::Schema(e.to_string()))
    }
}

pub fn load_manifest(path: &Path) -> Result<ParityManifest, ParityError> {
    let text = std::fs::read_to_string(path)?;
    parse_yaml(&text)
}

pub fn load_bundled() -> Result<ParityManifest, ParityError> {
    parse_yaml(BUNDLED)
}

fn parse_yaml(text: &str) -> Result<ParityManifest, ParityError> {
    let m: ParityManifest = serde_yaml::from_str(text)?;
    if m.version != 1 {
        return Err(ParityError::Schema(format!("version {} != 1", m.version)));
    }
    Ok(m)
}
```

`crates/cxas-parity/src/lib.rs`:

```rust
mod error;
mod manifest;

pub use error::ParityError;
pub use manifest::{
    load_bundled, load_manifest, Cli, IssueGate, ParityCommand, ParityEnum, ParityManifest,
    ParityMethod, ParityModule, ParityType, Source,
};
```

Minimal `parity/cxas-scrapi-parity.yaml` for this task (expanded in Task 2):

```yaml
version: 1
source:
  repository: GoogleCloudPlatform/cxas-scrapi
  commit: 4f7b43ca6adda0acad95a7e3654eee4e2ed1438c
modules: []
enums: []
cli:
  binary: cxas
  global_flags: [oauth-token, no-input]
  commands: []
issue_gates: []
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-parity --offline`
Expected: PASS (2 tests)

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/cxas-parity parity/cxas-scrapi-parity.yaml
git commit -m "feat(parity): load bundled cxas-scrapi parity manifest"
```

---

### Task 2: Freeze the public class, method, CLI, and issue-gate lists

**Files:**
- Modify: `parity/cxas-scrapi-parity.yaml`
- Modify: `crates/cxas-parity/src/manifest.rs` (duplicate detection)
- Test: `crates/cxas-parity/tests/manifest_contract.rs`

**Interfaces:**
- Consumes: `load_bundled`, `ParityManifest::require_type`, `require_command`, `issue_gates`
- Produces: complete YAML contents matching the spec tables; `parse_yaml` rejects duplicate `python_class` and duplicate CLI `argv`

- [ ] **Step 1: Write the failing test**

Append to `crates/cxas-parity/tests/manifest_contract.rs`:

```rust
const REQUIRED_CLASSES: &[&str] = &[
    "Agents", "Apps", "Callbacks", "Changelogs", "Common", "ConversationHistory",
    "Deployments", "Evaluations", "Guardrails", "Sessions", "Tools", "Variables",
    "Versions", "CallbackEvals", "GuardrailEvals", "SimulationEvals", "ToolEvals",
    "TurnEvals", "EvalUtils", "GoogleSheetsUtils", "SecretManagerUtils",
    "ChangelogUtils", "BaseDFCXClient", "ConversationalAgentsAPI",
    "DFCXAgentExporter", "DFCXAgents", "DFCXGenerativeSettings", "DFCXPlaybooks",
    "DFCXTools", "FlowDependencyResolver", "FlowTreeVisualizer",
    "HighLevelGraphVisualizer", "MainVisualizer", "PlaybookTreeVisualizer",
];

#[test]
fn every_frozen_python_class_is_present() {
    let m = load_bundled().unwrap();
    for class in REQUIRED_CLASSES {
        m.require_type(class)
            .unwrap_or_else(|_| panic!("missing class {class}"));
    }
}

#[test]
fn apps_sessions_evaluations_have_required_methods() {
    let m = load_bundled().unwrap();
    let apps = m.require_type("Apps").unwrap();
    for name in [
        "list_apps", "get_app", "export_app", "import_app", "import_as_new_app",
    ] {
        assert!(apps.methods.iter().any(|mm| mm.name == name), "{name}");
    }
    let evals = m.require_type("Evaluations").unwrap();
    assert!(evals
        .methods
        .iter()
        .any(|mm| mm.name == "wait_for_run_and_get_results"));
}

#[test]
fn frozen_cli_commands_are_present() {
    let m = load_bundled().unwrap();
    for argv in [
        &["pull"][..],
        &["push"],
        &["lint"],
        &["llm-lint"],
        &["evals", "report"],
        &["migrate", "dfcx"],
        &["trace"],
        &["init-github-action"],
    ] {
        m.require_command(argv)
            .unwrap_or_else(|_| panic!("missing {argv:?}"));
    }
}

#[test]
fn issue_gate_284_is_declared() {
    let m = load_bundled().unwrap();
    assert!(m.issue_gates().iter().any(|g| g.id == 284 && g.crate_name == "cxas-proto"));
}

#[test]
fn duplicate_class_is_rejected() {
    let yaml = r#"
version: 1
source: { repository: x, commit: y }
modules:
  - name: a
    rust_owner: cxas-core
    types:
      - { python_class: Apps, python_module: m, rust_type: Apps, methods: [] }
      - { python_class: Apps, python_module: m, rust_type: Apps, methods: [] }
enums: []
cli: { binary: cxas, commands: [] }
issue_gates: []
"#;
    let err = cxas_parity::parse_yaml_for_test(yaml).unwrap_err();
    assert!(matches!(err, ParityError::Duplicate(_)));
}
```

Export `parse_yaml_for_test` as `pub use manifest::parse_yaml` (rename `parse_yaml` to public `pub fn parse_yaml`).

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-parity every_frozen_python_class_is_present --offline`
Expected: FAIL with `missing class Agents` (or the first required class)

- [ ] **Step 3: Write minimal implementation**

1. Make `parse_yaml` public and after deserialize walk types/commands to reject duplicates:

```rust
pub fn parse_yaml(text: &str) -> Result<ParityManifest, ParityError> {
    let m: ParityManifest = serde_yaml::from_str(text)?;
    if m.version != 1 {
        return Err(ParityError::Schema(format!("version {} != 1", m.version)));
    }
    let mut seen_types = std::collections::BTreeSet::new();
    for module in &m.modules {
        for t in &module.types {
            if !seen_types.insert(t.python_class.clone()) {
                return Err(ParityError::Duplicate(t.python_class.clone()));
            }
        }
    }
    let mut seen_cmd = std::collections::BTreeSet::new();
    for c in &m.cli.commands {
        if !seen_cmd.insert(c.argv.clone()) {
            return Err(ParityError::Duplicate(c.argv.join(" ")));
        }
    }
    Ok(m)
}
```

2. Expand `parity/cxas-scrapi-parity.yaml` so every class in `REQUIRED_CLASSES` appears under the `rust_owner` from the spec table, every required method is listed, every CLI argv from the spec table is listed (including `agent`, `tool`, `guardrail`, `apps list`, `conversations get`, `deployments promote`, `local create`, `versions compare`, `insights`), and `issue_gates` contains:

```yaml
issue_gates:
  - id: 284
    crate_name: cxas-proto
    test: evaluation_run_state_unknown_variant_is_typed
```

(`crate_name` not `crate` — `crate` is a reserved YAML/Rust word; the spec field `crate` is serialized as `crate_name`.)

Also add the `EvaluationRunState` enum entry:

```yaml
enums:
  - python_name: EvaluationRunState
    proto_type: google.cloud.ces.v1.EvaluationRunState
    rust_type: cxas_proto::EvaluationRunState
    rust_owner: cxas-proto
    unknown_policy: retain_wire_value
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-parity --offline`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add parity/cxas-scrapi-parity.yaml crates/cxas-parity
git commit -m "feat(parity): freeze cxas-scrapi class, CLI, and #284 issue gate"
```

---

### Task 3: `EvaluationRunState` unknown-variant wrapper (#284)

**Files:**
- Create: `proto/ces/evaluation_run_state.proto`
- Create: `crates/cxas-proto/Cargo.toml`
- Create: `crates/cxas-proto/build.rs`
- Create: `crates/cxas-proto/src/lib.rs`
- Create: `crates/cxas-proto/src/evaluation_run_state.rs`
- Test: `crates/cxas-proto/tests/unknown_state.rs`
- Modify: root `Cargo.toml` members to include `crates/cxas-proto`

**Interfaces:**
- Consumes: nothing from `cxas-parity` at runtime
- Produces: `pub enum EvaluationRunState { Unspecified, Pending, Running, Succeeded, Failed, Cancelled, Unknown(i32) }`, `from_wire(i32) -> Self`, `as_str_name(&self) -> Cow<str>`

- [ ] **Step 1: Write the failing test**

```rust
// crates/cxas-proto/tests/unknown_state.rs
use cxas_proto::EvaluationRunState;

#[test]
fn unknown_wire_value_is_typed_not_a_panic() {
    let state = EvaluationRunState::from_wire(99);
    assert_eq!(state, EvaluationRunState::Unknown(99));
    assert_eq!(state.as_str_name(), "UNKNOWN(99)");
}

#[test]
fn known_wire_value_maps() {
    assert_eq!(EvaluationRunState::from_wire(3), EvaluationRunState::Succeeded);
    assert_eq!(EvaluationRunState::Succeeded.as_str_name(), "SUCCEEDED");
}

#[test]
fn source_never_calls_name_on_i32() {
    let src = include_str!("../src/evaluation_run_state.rs");
    assert!(
        !src.contains(".name()"),
        "reintroduces the Python #284 crash class"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-proto unknown_wire_value_is_typed_not_a_panic --offline`
Expected: FAIL with `could not find package cxas-proto`

- [ ] **Step 3: Write minimal implementation**

`proto/ces/evaluation_run_state.proto`:

```proto
syntax = "proto3";
package google.cloud.ces.v1;

enum EvaluationRunState {
  EVALUATION_RUN_STATE_UNSPECIFIED = 0;
  PENDING = 1;
  RUNNING = 2;
  SUCCEEDED = 3;
  FAILED = 4;
  CANCELLED = 5;
}
```

`crates/cxas-proto/src/evaluation_run_state.rs`:

```rust
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvaluationRunState {
    Unspecified,
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Unknown(i32),
}

impl EvaluationRunState {
    pub fn from_wire(value: i32) -> Self {
        match value {
            0 => Self::Unspecified,
            1 => Self::Pending,
            2 => Self::Running,
            3 => Self::Succeeded,
            4 => Self::Failed,
            5 => Self::Cancelled,
            other => Self::Unknown(other),
        }
    }

    pub fn as_str_name(&self) -> Cow<'static, str> {
        match self {
            Self::Unspecified => Cow::Borrowed("EVALUATION_RUN_STATE_UNSPECIFIED"),
            Self::Pending => Cow::Borrowed("PENDING"),
            Self::Running => Cow::Borrowed("RUNNING"),
            Self::Succeeded => Cow::Borrowed("SUCCEEDED"),
            Self::Failed => Cow::Borrowed("FAILED"),
            Self::Cancelled => Cow::Borrowed("CANCELLED"),
            Self::Unknown(n) => Cow::Owned(format!("UNKNOWN({n})")),
        }
    }
}
```

`crates/cxas-proto/src/lib.rs`:

```rust
mod evaluation_run_state;
pub use evaluation_run_state::EvaluationRunState;
```

`crates/cxas-proto/Cargo.toml`:

```toml
[package]
name = "cxas-proto"
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"
rust-version = "1.80"

[dependencies]
prost = "0.13"
tonic = { version = "0.12", default-features = false, features = ["codegen", "prost"] }

[build-dependencies]
tonic-build = "0.12"
```

`build.rs` compiles `../../proto/ces/evaluation_run_state.proto` into `OUT_DIR` even if the wrapper is hand-written; keep generation so later protos have a path.

Add `"crates/cxas-proto"` to workspace members.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-proto --offline`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add proto/ces crates/cxas-proto Cargo.toml
git commit -m "feat(proto): type unknown EvaluationRunState wire values (#284)"
```

---

### Task 4: Exhaustive-match compile-fail fixture

**Files:**
- Create: `crates/cxas-proto/trybuild/fail_missing_unknown.rs`
- Create: `crates/cxas-proto/tests/trybuild.rs`
- Modify: `crates/cxas-proto/Cargo.toml` (dev-dep `trybuild`)

**Interfaces:**
- Consumes: `EvaluationRunState`
- Produces: a compile-fail test proving a `match` without `Unknown` does not compile

- [ ] **Step 1: Write the failing test**

`crates/cxas-proto/tests/trybuild.rs`:

```rust
#[test]
fn missing_unknown_arm_does_not_compile() {
    let t = trybuild::TestCases::new();
    t.compile_fail("trybuild/fail_missing_unknown.rs");
}
```

`crates/cxas-proto/trybuild/fail_missing_unknown.rs`:

```rust
fn label(s: cxas_proto::EvaluationRunState) -> &'static str {
    match s {
        cxas_proto::EvaluationRunState::Unspecified => "u",
        cxas_proto::EvaluationRunState::Pending => "p",
        cxas_proto::EvaluationRunState::Running => "r",
        cxas_proto::EvaluationRunState::Succeeded => "s",
        cxas_proto::EvaluationRunState::Failed => "f",
        cxas_proto::EvaluationRunState::Cancelled => "c",
    }
}

fn main() {
    let _ = label(cxas_proto::EvaluationRunState::Pending);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-proto missing_unknown_arm_does_not_compile --offline`
Expected: FAIL because `trybuild` is not in `[dev-dependencies]` (or the test file is missing)

- [ ] **Step 3: Write minimal implementation**

Add to `crates/cxas-proto/Cargo.toml`:

```toml
[dev-dependencies]
trybuild = "1"
```

No production code change; the type already has `Unknown`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-proto --offline`
Expected: PASS, trybuild reports the fixture failed to compile with `non-exhaustive patterns`

- [ ] **Step 5: Commit**

```bash
git add crates/cxas-proto
git commit -m "test(proto): require exhaustive match on EvaluationRunState"
```
