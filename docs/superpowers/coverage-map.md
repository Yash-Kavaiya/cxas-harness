# Superpowers coverage map

Maps every `PRD.md.txt` / `dev.md.txt` development phase and every cataloged `GoogleCloudPlatform/cxas-scrapi` open issue (25 as of 2026-08-15, SHA `4f7b43ca6adda0acad95a7e3654eee4e2ed1438c`) onto a design spec section and an implementation-plan task.

Source of truth for "done" of the spec/plan set: this file plus the files it names.

## Phase map

| Phase | Source-doc title | Spec | Plan |
|---|---|---|---|
| 0 | Foundations and parity contract | `docs/superpowers/specs/2026-08-15-parity-contract-design.md` | `docs/superpowers/plans/2026-08-15-parity-contract.md` |
| 1 | Crate architecture / workspace | `docs/superpowers/specs/2026-08-15-workspace-architecture-design.md` | `docs/superpowers/plans/2026-08-15-workspace-architecture.md` |
| 2 | Bidi-streaming and eval correctness | `docs/superpowers/specs/2026-08-15-bidi-eval-correctness-design.md` | `docs/superpowers/plans/2026-08-15-bidi-eval-correctness.md` |
| 3 | Lint and validation engine | `docs/superpowers/specs/2026-08-15-lint-validation-design.md` | `docs/superpowers/plans/2026-08-15-lint-validation.md` |
| 4 | Migration and eval-run cleanup | `docs/superpowers/specs/2026-08-15-migration-lifecycle-design.md` | `docs/superpowers/plans/2026-08-15-migration-lifecycle.md` |
| 5 | Packaging, docs, and release | `docs/superpowers/specs/2026-08-15-packaging-cli-ci-design.md` | `docs/superpowers/plans/2026-08-15-packaging-cli-ci.md` |

Gauntlet Loop is implemented as repo tooling under `gauntlet/`, specified in
`docs/superpowers/specs/2026-08-15-discovery-benchmark-gauntlet-design.md` and
planned in `docs/superpowers/plans/2026-08-15-discovery-benchmark-gauntlet.md`.
It is non-runtime: nothing under `gauntlet/` is a Cargo workspace member.

## Source-doc requirement map

| Source requirement | Spec section | Plan task |
|---|---|---|
| Parity contract YAML/JSON of Python public surface | Phase 0 **Components** (`parity/cxas-scrapi-parity.yaml`) | Phase 0 Task 1, Task 2 |
| Vendor CES protos + tonic/prost bindings | Phase 0 **Components** (`proto/ces`, `cxas-proto`) | Phase 0 Task 3, Task 4 |
| Workspace crates: proto, core, utils, evals, migration, cli, state | Phase 1 **Architecture** and **Components** | Phase 1 Tasks 1–6 |
| `location` mandatory typed field, never silent `"global"` | Phase 1 **Components** `Location`; Phase 0 **Global constraints** | Phase 1 Task 1 |
| Feature-flagged utils (Sheets/BQ/audio) | Phase 1 **Components** `cxas-utils` | Phase 1 Task 4 |
| Typed simulation turn state machine | Phase 2 **Components** `TurnState`, `TurnCursor` | Phase 2 Task 1, Task 2 |
| Dedicated Evaluation `RunSession` quota client | Phase 1 `QuotaKind`; Phase 2 `SimulationEvals` | Phase 1 Task 3; Phase 2 Task 3 |
| `tokio::select!` bidi handler | Phase 2 **Components** `BidiSession` | Phase 2 Task 2 |
| Pluggable audio scorer / SpeechPath | Phase 2 **Components** `AudioScorer` | Phase 2 Task 4 |
| Lint rule registry + completeness vs schema | Phase 3 **Components** `RuleRegistry` | Phase 3 Task 2 |
| `llm-lint` Gemini HTTP client | Phase 3 **Components** `LlmLintClient` | Phase 3 Task 4 |
| RAII snapshot cleanup | Phase 4 **Components** `SnapshotGuard` | Phase 4 Task 1 |
| Tool-deletion sync via state hash | Phase 4 **Components** `ToolSync` | Phase 4 Task 2 |
| Non-interactive migration default + TUI feature | Phase 4 **Components** `MigrationPipeline` | Phase 4 Task 3 |
| Static binary / cargo-dist | Phase 5 **Components** Packaging | Phase 5 Task 5 (`dist-workspace.toml`) |
| mdBook Docs/Examples/Agent Skills/Core SDK | Phase 5 **Components** Docs | Phase 5 Task 5 |
| `cxas actions init` GitHub Actions | Phase 5 **Components** `actions init` | Phase 5 Task 2 |
| Machine-first CLI JSON + stable exits | Phase 5 **Components** `cxas-cli` | Phase 5 Task 1 |

## Cataloged issue map (all 25)

| Issue | Title (short) | Spec section | Plan task |
|---|---|---|---|
| #27 | Audio Evaluations | Phase 2 **Components** `AudioScorer` | Phase 2 Task 4 |
| #46 | Create Formal Github Actions | Phase 5 **Components** `actions init` | Phase 5 Task 2 |
| #54 | Multi-environment GHA via `environment.json` | Phase 5 **Components** `actions init` | Phase 5 Task 2 |
| #55 | CLI ergonomics for coding agents | Phase 5 **Architecture** / `cxas-cli` JSON envelope | Phase 5 Task 1 |
| #86 | Missing root-agent validation in `cxas lint` | Phase 3 **Components** `V-ROOT` | Phase 3 Task 1 |
| #99 | Dependency Dashboard | Phase 5 **Components** `deny.toml` | Phase 5 Task 5 |
| #131 | RFC `cxas diff` / `cxas state` | Phase 1 `cxas-state`; Phase 5 `diff`/`state` | Phase 1 Task 5; Phase 5 Task 4 |
| #136 | Voice simulations ignore returned audio | Phase 2 **Components** `AudioScorer` / `MissingAgentAudio` | Phase 2 Task 4 |
| #168 | Hillclimbing leaves agent snapshots | Phase 4 **Components** `SnapshotGuard` | Phase 4 Task 1 |
| #188 | SpeechPath | Phase 2 **Components** `SpeechPathScorer` | Phase 2 Task 4 |
| #206 | Turn evals in combined HTML/JSON report | Phase 2 **Components** `TurnRow`; Phase 5 `evals report` | Phase 2 Task 5; Phase 5 Task 3 |
| #252 | `cxas pull --version-id` | Phase 1 `Apps::export_app_version`; Phase 5 `pull` | Phase 1 Task 2; Phase 5 Task 3 |
| #256 | Boolean types in environment templates | Phase 1 `render_environment` / `TemplateValue::Bool` | Phase 1 Task 4 |
| #263 | Evaluation `RunSession` quota | Phase 1 `QuotaKind`; Phase 2 `SimulationEvals` | Phase 1 Task 3; Phase 2 Task 3 |
| #270 | Unified workspace resolution & cascading profiles | Phase 1 `resolve_workspace`; Phase 5 `state` | Phase 1 Task 5; Phase 5 Task 4 |
| #284 | `EvaluationRunState` stub drift / `.name` on int | Phase 0 **Components** `EvaluationRunState::Unknown` | Phase 0 Task 3, Task 4 |
| #298 | Pull apps ≥ 4 MB | Phase 1 streamed `ExportHandle`; Phase 5 `pull` | Phase 1 Task 2; Phase 5 Task 3 |
| #345 | DTMF hang: bidi never waits for agent | Phase 2 **Components** `BidiSession::drive_turn` | Phase 2 Task 2 |
| #350 | `cxas trace` per-turn raw JSON | Phase 5 **Components** `trace --raw` | Phase 5 Task 3 |
| #355 | Simulation repeats first `static_utterance` | Phase 2 **Components** `TurnCursor` | Phase 2 Task 1, Task 3 |
| #386 | Native `cxas deploy` | Phase 5 **Components** `deploy` | Phase 5 Task 4 |
| #394 | Cloud tool deletion not reflected locally | Phase 4 **Components** `ToolSync` | Phase 4 Task 2 |
| #397 | Web Widget welcome-event + deployment-version validation | Phase 3 **Components** `V-WELCOME`, `V-DEPVER` | Phase 3 Task 3 |
| #401 | Hardcoded `"global"` `vertex_location` (Blocker) | Phase 1 **Components** `Location`; no `Default` | Phase 1 Task 1 |
| #403 | Deployment channel settings (noise cancellation) | Phase 1 **Components** `ChannelSettings` | Phase 1 Task 3 |

Count: 25 issue rows. Each row names one spec file (via its phase) and at least one plan task with a failing-test step.

## Quality bar

`cxas-harness` is accepted when (a) every enum and method the Rust crates
declare resolves against the vendored CES discovery documents under
`reference/ces/`, and (b) every issue in the table has a closing test that
exercises behaviour verified against discovery rather than against a test
double asserting the code's own assumptions.

Clause (a) replaces the former parity-manifest bar, which was self-graded: it
asserted that a checked-in YAML contained strings that same YAML declared, so
it could never fail. It did not, for example, notice that
`EvaluationRunState` declared `PENDING`/`SUCCEEDED`/`FAILED` where CES declares
`QUEUED`/`COMPLETED`/`ERROR`. The Python parity manifest is retained as a
CLI-shape reference only.
