# Phase 0 — Foundations and Parity Contract Design

**Date:** 2026-08-15
**Status:** Approved from the product briefs (retired 2026-08-16; requirements restated here) (no additional product Q&A)
**Product:** `cxas-harness` (Rust rewrite of Python `cxas-scrapi`)
**Phase:** 0 of 5 — foundations and parity contract
**Grounding snapshot:** `GoogleCloudPlatform/cxas-scrapi` SHA `4f7b43ca6adda0acad95a7e3654eee4e2ed1438c` (25 open issues as of 2026-08-15)

## Purpose

`cxas-harness` is a correctness- and ergonomics-improved Rust rewrite of Google Cloud Platform's `cxas-scrapi` Python SDK, CLI, and Agent Skills toolkit for CX Agent Studio (CES). Before any product crate ships behavior, this phase freezes a machine-readable **parity contract** of the public Python surface and vendors CES protobuf definitions so enum drift cannot reach runtime.

This spec is independently shippable: its output is a checked-in YAML manifest, a loader crate that validates that manifest, vendored `.proto` files, and generated `cxas-proto` bindings. Later phases consume those artifacts; they do not redefine the public surface.

**Issue-driven quality bar for this spec:** every later crate is scored against (a) behavioral parity with the Python `cxas-scrapi` surface enumerated here and (b) a pass/fail closing test for each of the 25 cataloged GitHub issues. This phase itself closes **#284** (`wait_for_run_and_get_results` crashes with `'int' object has no attribute 'name'` when the server returns an `EvaluationRunState` the installed stubs do not define) by construction: generated Rust enums are exhaustive, unknown wire values become a typed `Unknown(i32)` variant, and no code path calls `.name` on a raw integer.

Gauntlet Loop builder/critic orchestration is an out-of-band process overlay described in the source product docs. It is not the methodology of this Superpowers spec set and is not a runtime dependency of `cxas-harness`.

## Architecture

The Phase 0 system is a **contract-plus-bindings** layer with no CES network I/O.

```
Python cxas-scrapi (read-only reference)
        |
        v
parity/cxas-scrapi-parity.yaml   <-- frozen public surface
        |
        v
crates/cxas-parity               <-- load, validate, query the contract
        |
        +--> later crates (cxas-core, cxas-evals, cxas-cli, ...)
                 each crate lists the manifest entries it owns

Vendored proto/ces/*.proto
        |
        v
crates/cxas-proto (tonic + prost)
        |
        +--> EvaluationRunState and every other CES enum
```

Two independent artifacts, one workspace:

1. **Parity manifest** — YAML (canonical) with a JSON-equivalent produced by the loader for machine consumers. It enumerates every public class, method, and CLI subcommand that `cxas-harness` must provide an equivalent for.
2. **Protobuf bindings** — vendored CES / Conversational Agents protos compiled with `tonic-build` / `prost`. Generation is deterministic and gated by CI: if the vendored protos change, the generated Rust must change in the same commit.

`cxas-parity` never talks to CES. `cxas-proto` never interprets the parity manifest. Coupling is one-way: later crates depend on both.

## Components

### 1. `parity/cxas-scrapi-parity.yaml`

Canonical contract file. Schema:

```yaml
version: 1
source:
  repository: GoogleCloudPlatform/cxas-scrapi
  commit: 4f7b43ca6adda0acad95a7e3654eee4e2ed1438c
modules:
  - name: cxas_scrapi.core
    rust_owner: cxas-core
    types: [...]
  - name: cxas_scrapi.evals
    rust_owner: cxas-evals
    types: [...]
  - name: cxas_scrapi.cli
    rust_owner: cxas-cli
    commands: [...]
  - name: cxas_scrapi.migration
    rust_owner: cxas-migration
    types: [...]
  - name: cxas_scrapi.utils
    rust_owner: cxas-utils
    types: [...]
enums:
  - python_name: EvaluationRunState
    proto_type: google.cloud.ces.v1.EvaluationRunState
    rust_type: cxas_proto::ces::v1::EvaluationRunState
    rust_owner: cxas-proto
    unknown_policy: retain_wire_value
cli:
  binary: cxas
  global_flags: [oauth-token, no-input]
  commands: [...]
issue_gates:
  - id: 284
    crate: cxas-proto
    test: evaluation_run_state_unknown_variant_is_typed
```

Every `types[]` entry has `python_class`, `python_module`, `methods[]` (each with `name`, `params[]`, `returns`), and `rust_type`. Every CLI command has `argv` (e.g. `["pull"]`, `["evals", "report"]`), `python_handler`, and `rust_owner`.

Frozen public Python types that **must** appear in the manifest (from `cxas_scrapi.__all__` at the grounding snapshot):

| Python class | Python module | Rust owner crate |
|---|---|---|
| `Agents` | `cxas_scrapi.core.agents` | `cxas-core` |
| `Apps` | `cxas_scrapi.core.apps` | `cxas-core` |
| `Callbacks` | `cxas_scrapi.core.callbacks` | `cxas-core` |
| `Changelogs` | `cxas_scrapi.core.changelogs` | `cxas-core` |
| `Common` | `cxas_scrapi.core.common` | `cxas-core` |
| `ConversationHistory` | `cxas_scrapi.core.conversation_history` | `cxas-core` |
| `Deployments` | `cxas_scrapi.core.deployments` | `cxas-core` |
| `Evaluations` | `cxas_scrapi.core.evaluations` | `cxas-core` |
| `Guardrails` | `cxas_scrapi.core.guardrails` | `cxas-core` |
| `Sessions` | `cxas_scrapi.core.sessions` | `cxas-core` |
| `Tools` | `cxas_scrapi.core.tools` | `cxas-core` |
| `Variables` | `cxas_scrapi.core.variables` | `cxas-core` |
| `Versions` | `cxas_scrapi.core.versions` | `cxas-core` |
| `CallbackEvals` | `cxas_scrapi.evals.callback_evals` | `cxas-evals` |
| `GuardrailEvals` | `cxas_scrapi.evals.guardrail_evals` | `cxas-evals` |
| `SimulationEvals` | `cxas_scrapi.evals.simulation_evals` | `cxas-evals` |
| `ToolEvals` | `cxas_scrapi.evals.tool_evals` | `cxas-evals` |
| `TurnEvals` | `cxas_scrapi.evals.turn_evals` | `cxas-evals` |
| `EvalUtils` | `cxas_scrapi.utils.eval_utils` | `cxas-utils` |
| `GoogleSheetsUtils` | `cxas_scrapi.utils.google_sheets_utils` | `cxas-utils` |
| `SecretManagerUtils` | `cxas_scrapi.utils.secret_manager_utils` | `cxas-utils` |
| `ChangelogUtils` | `cxas_scrapi.utils.changelog_utils` | `cxas-utils` |
| `BaseDFCXClient` | `cxas_scrapi.migration.dfcx_exporter` | `cxas-migration` |
| `ConversationalAgentsAPI` | `cxas_scrapi.migration.dfcx_exporter` | `cxas-migration` |
| `DFCXAgentExporter` | `cxas_scrapi.migration.dfcx_exporter` | `cxas-migration` |
| `DFCXAgents` | `cxas_scrapi.migration.dfcx_exporter` | `cxas-migration` |
| `DFCXGenerativeSettings` | `cxas_scrapi.migration.dfcx_exporter` | `cxas-migration` |
| `DFCXPlaybooks` | `cxas_scrapi.migration.dfcx_exporter` | `cxas-migration` |
| `DFCXTools` | `cxas_scrapi.migration.dfcx_exporter` | `cxas-migration` |
| `FlowDependencyResolver` | `cxas_scrapi.migration.flow_visualizer` | `cxas-migration` |
| `FlowTreeVisualizer` | `cxas_scrapi.migration.flow_visualizer` | `cxas-migration` |
| `HighLevelGraphVisualizer` | `cxas_scrapi.migration.graph_visualizer` | `cxas-migration` |
| `MainVisualizer` | `cxas_scrapi.migration.main_visualizer` | `cxas-migration` |
| `PlaybookTreeVisualizer` | `cxas_scrapi.migration.playbook_visualizer` | `cxas-migration` |

Minimum method sets that the manifest **must** record (Python names; Rust equivalents use `snake_case` of the same verb):

- `Apps`: `list_apps`, `get_app`, `get_app_by_display_name`, `create_app`, `delete_app`, `export_app`, `import_app`, `import_as_new_app`, `get_apps_map`
- `Agents`: `get_agents_map`, `list_agents`, `get_agent`, `create_agent`, `update_agent`, `delete_agent`
- `Tools`: `get_tools_map`, `list_tools`, `get_tool`, `create_tool`, `update_tool`, `delete_tool`
- `Guardrails`: `list_guardrails`, `get_guardrail`, `create_guardrail`, `update_guardrail`, `delete_guardrail`
- `Deployments`: `list_deployments`, `get_deployment`, `create_deployment`, `update_deployment`, `delete_deployment`
- `Sessions`: `create_session_id`, `run`, `parse_result`, `bidi_run` (Python `BidiSessionHandler` surface)
- `Evaluations`: `list_evaluations`, `get_evaluation`, `update_evaluation`, `run_evaluation`, `export_evaluation`, `get_evaluation_result`, `wait_for_run_and_get_results`, `get_evaluations_map`
- `SimulationEvals`: `run_simulations` (must consume a turn cursor, not a single `static_utterance`)
- `Versions`: `list_versions`, `create_version`, `compare_versions`, `get_version`

Frozen CLI surface (Python `cxas` entry point at the grounding snapshot; `cxas-cli` must expose each as a clap subcommand):

| argv | Python handler module |
|---|---|
| `migrate dfcx` | `cli.main.run_migration_dashboard` |
| `init-github-action` | `core.github.init_github_action` |
| `evals report` | `cli.main.combined_evals_report_cmd` |
| `test-tools` | `cli.main.test_tools` |
| `test-callbacks` | `cli.main.test_callbacks` |
| `test-single-callback` | `cli.main.test_single_callback` |
| `export` | `cli.main.export_eval` |
| `push-eval` | `cli.main.push_eval` |
| `run` | `cli.main.run_eval` |
| `run-session` | `cli.main.run_session` |
| `ci-test` | `cli.main.ci_test` |
| `local-test` | `cli.main.local_test` |
| `delete` | `cli.app.app_delete` |
| `pull` | `cli.app.app_pull` |
| `push` | `cli.app.app_push` |
| `lint` | `cli.app.app_lint` |
| `llm-lint` | `cli.llm_lint.llm_lint` |
| `help` | `cli.main.cmd_help` |
| `init` | `cli.app.app_init` |
| `create` | `cli.app.app_create` |
| `branch` | `cli.app.app_branch` |
| `apps list` | `cli.app.apps_list` |
| `apps get` | `cli.app.apps_get` |
| `conversations list` | `cli.main.conversations_list` |
| `conversations get` | `cli.main.conversations_get` |
| `deployments list` | `cli.main.deployments_list` |
| `deployments create` | `cli.main.deployments_create` |
| `deployments promote` | `cli.main.deployments_promote` |
| `local create` | `cli.create_local.handle_local_create` |
| `versions list` | `cli.versions_cli.app_versions_list` |
| `versions compare` | `cli.versions_cli.app_versions_compare` |
| `insights` | `cli.insights_cli` |
| `trace` | `cli.trace_cli` |
| resource verbs `agent`, `tool`, `guardrail` | `cli.resources_cli` |

Phase 5 adds new subcommands (`actions init`, `diff`, `state`, `deploy`) that have no Python equivalent; they are **extensions**, recorded in the manifest under `extensions[]` with `parity: additive` so critics do not treat them as missing Python surface.

### 2. `crates/cxas-parity`

Library crate. Public API:

```rust
pub struct ParityManifest { /* owned tree matching the YAML schema */ }
pub fn load_manifest(path: &Path) -> Result<ParityManifest, ParityError>;
pub fn load_bundled() -> Result<ParityManifest, ParityError>;
impl ParityManifest {
    pub fn types_for_crate(&self, rust_owner: &str) -> Vec<&ParityType>;
    pub fn commands_for_crate(&self, rust_owner: &str) -> Vec<&ParityCommand>;
    pub fn require_type(&self, python_class: &str) -> Result<&ParityType, ParityError>;
    pub fn require_command(&self, argv: &[&str]) -> Result<&ParityCommand, ParityError>;
    pub fn issue_gates(&self) -> &[IssueGate];
    pub fn to_json(&self) -> Result<String, ParityError>;
}
```

The crate embeds `parity/cxas-scrapi-parity.yaml` via `include_str!` so `load_bundled()` works in tests without a repo-relative path.

### 3. `proto/ces/` and `crates/cxas-proto`

Vendored CES protobufs live under `proto/ces/` (not generated into `target/`). `crates/cxas-proto/build.rs` invokes `tonic_build` with:

- `compile_well_known_types` for `google.protobuf.*`
- `type_attribute` on every enum forcing `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`
- A post-generation patch that, for every proto enum including `EvaluationRunState`, emits an `Unknown(i32)` variant (or uses prost's `#[prost(enumeration)]` plus a wrapper)

Public wrapper required for #284:

```rust
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
    pub fn from_wire(value: i32) -> Self { /* map known; else Unknown(value) */ }
    pub fn as_str_name(&self) -> Cow<'static, str> {
        // never panics; Unknown returns "UNKNOWN(<n>)"
    }
}
```

`wait_for_run_and_get_results` (implemented in Phase 1 `cxas-core`) **must** match on `EvaluationRunState` and treat `Unknown(_)` as a first-class, reportable state. It must not assume every wire integer has a generated name.

### 4. Completeness extractor (dev-only)

`scripts/extract_python_surface.py` is a reference helper that walks a local checkout of `cxas-scrapi` and prints the public class/method/CLI set. It is **not** run in `cxas-harness` CI (the Python tree is not a runtime dependency). The checked-in YAML is the source of truth; the script is for regenerating a candidate when the grounding snapshot is deliberately bumped.

## Data flow

1. A maintainer (or Phase 0 implementer) reads `cxas-scrapi` at the pinned commit and writes/updates `parity/cxas-scrapi-parity.yaml`.
2. `cxas-parity` unit tests load the bundled YAML, reject schema errors, and assert the frozen class list, method minima, and CLI argv list above.
3. A later crate's test calls `ParityManifest::types_for_crate("cxas-core")` and asserts every listed method has a corresponding Rust item (those assertions land in later-phase plans).
4. `cxas-proto/build.rs` compiles `proto/ces/*.proto` into Rust. Tests decode a crafted `EvaluationRun` protobuf whose `state` field is an integer that is **not** in the known enum and assert `EvaluationRunState::Unknown(n)` plus a non-panicking `as_str_name()`.
5. CI runs `cargo test -p cxas-parity -p cxas-proto` and a proto-lock check: `git diff --exit-code proto/ crates/cxas-proto/src/generated/` after a dry generation.

No CES credentials, no network, no filesystem writes except the generator's output directory.

## Error handling

| Condition | Type | Behavior |
|---|---|---|
| YAML missing / unreadable | `ParityError::Io` | `load_manifest` returns `Err`; tests fail closed |
| YAML fails schema (missing `version`, `modules`, `cli`, `enums`) | `ParityError::Schema` | `Err` with field path |
| Duplicate `python_class` or duplicate CLI `argv` | `ParityError::Duplicate` | `Err`; manifest is invalid |
| `require_type` / `require_command` miss | `ParityError::Missing { name }` | `Err`; callers do not unwrap |
| Proto compile failure | build.rs `panic` | crate does not compile |
| Unknown enum wire value | `EvaluationRunState::Unknown(i32)` | success path; never a panic, never `.name` on `i32` |
| `as_str_name` on `Unknown(n)` | `Cow::Owned(format!("UNKNOWN({n})"))` | stable string for logs/JSON |

`ParityError` and proto decode errors implement `std::error::Error` + `Display`. Library code uses `Result`; only `build.rs` may panic, and only on generation failure.

CLI global flags inherited from Python and recorded in the manifest: `--oauth-token` and `--no-input`. `--no-input` is the default in `cxas-harness` (machine-first); the Python default is interactive. That default flip is an intentional, documented incompatibility, stored on the `cli` node as `defaults.no_input: true`.

## Testing

All Phase 0 tests are unit tests. They do not call CES.

1. **Manifest schema** — `load_bundled()` succeeds; `version == 1`; `source.commit` equals the grounding SHA.
2. **Frozen class set** — every class in the table above is present exactly once; `require_type("Apps")` returns the Apps entry.
3. **Frozen method minima** — `Apps`, `Sessions`, `Evaluations`, `SimulationEvals` contain the methods listed above.
4. **Frozen CLI set** — every argv row above is present; `require_command(&["lint"])` and `require_command(&["evals", "report"])` succeed.
5. **Issue gate #284 is declared** — `issue_gates` contains `id: 284` owned by `cxas-proto`.
6. **Unknown `EvaluationRunState`** — decode wire value `99` (or any unused integer) → `Unknown(99)`; `as_str_name()` equals `"UNKNOWN(99)"`; a `match` without a catch-all does not compile (documented by a `trybuild` compile-fail fixture that omits `Unknown`).
7. **No panic on name** — a helper `fn state_label(state: EvaluationRunState) -> String { state.as_str_name().into_owned() }` is the only supported way to print a state; a grep-style unit test on `crates/cxas-proto` source rejects any `.name` call on a raw `i32`.
8. **JSON round-trip** — `to_json()` parses back to an equal `ParityManifest`.

`cxas-scrapi` parity for this phase means: the manifest is a complete, machine-readable projection of the Python public surface at the pinned commit. It does not yet implement that surface.

## Global constraints (inherited by later specs)

- Language: Rust 2021 edition, MSRV 1.80.
- Async runtime: `tokio` (full) only in crates that perform I/O; `cxas-parity` is sync.
- gRPC/protobuf: `tonic` + `prost` only; no Python protobuf stubs.
- `location` is never defaulted to `"global"` (enforced in Phase 1; Phase 0 records the constraint).
- Feature flags isolate optional integrations (Sheets, BigQuery, TUI, audio).
- Machine-first CLI: structured JSON, stable exit codes, non-interactive by default.
- Issue-driven quality bar: 25 cataloged `GoogleCloudPlatform/cxas-scrapi` issues each require a closing test before release candidate.
- Apache-2.0 license headers on every new Rust file.
- No Gauntlet Loop runtime; Superpowers spec→plan is the development process for this repository.

## Out of scope

- Implementing `Apps` / `Sessions` / eval runners (Phase 1–2).
- Compiling a CES client that dials a real endpoint.
- Porting the Python pytest suite into running tests.
- Closing issues other than declaring the #284 gate and the enum type that closes it.
- Interactive human approval of individual design sections (source docs are the signed-off input).
