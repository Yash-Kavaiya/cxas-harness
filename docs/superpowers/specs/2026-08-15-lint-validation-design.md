# Phase 3 — Lint and Validation Engine Design

**Date:** 2026-08-15
**Status:** Approved from `PRD.md.txt` and `dev.md.txt`
**Product:** `cxas-harness`
**Phase:** 3 of 5 — lint and validation
**Depends on:** Phase 0 parity (`lint`, `llm-lint` commands), Phase 1 `cxas-state` / `cxas-utils`

## Purpose

Port `cxas lint` to a rule-registry crate (`cxas-lint`) where every check is a `LintRule` with its own unit tests, and port `cxas llm-lint` as a thin Gemini HTTP client. A completeness test diffs the registry against the app-schema required fields so missing rules cannot silently ship.

**Issue-driven quality bar:** this phase closes **#86** (missing root-agent validation in `cxas lint` causes failed pushes) and **#397** (Web Widget welcome-event and deployment-version validation guidance — encoded as lint rules, not prose-only docs). `cxas-scrapi` parity means `cxas lint` and `cxas llm-lint` accept the same path inputs (`--app-dir`, per-resource flags) and produce a machine-readable diagnostics list. The Python linter's 60+ structural/schema checks are the coverage floor; the registry length must be ≥ 60, and every required field in `schema/app.required.json` must have a rule id.

## Architecture

```
app directory (YAML/JSON + instruction.txt)
        |
        v
Discovery  -->  LintContext (files, parsed docs, schema)
        |
        v
RuleRegistry  --run_all-->  Vec<Diagnostic>
        |
        +--> JSON writer (default in cxas-harness)
        +--> human writer (optional --human)

instruction.txt / global_instruction.txt
        |
        v
LlmLintClient (Gemini HTTP) --> semantic diagnostics
```

Rules are pure functions of `LintContext`. They do not I/O except through the context already loaded. `llm-lint` is a separate binary path: it may call the network, and it is feature-gated (`llm`).

## Components

### 1. `LintRule` trait

```rust
pub trait LintRule: Send + Sync {
    fn id(&self) -> &'static str;          // e.g. "V001", "V-ROOT"
    fn description(&self) -> &'static str;
    fn applies_to(&self) -> RuleScope;
    fn run(&self, ctx: &LintContext) -> Vec<Diagnostic>;
}

pub enum RuleScope { App, Agent, Tool, Guardrail, Example, Evaluation, Deployment }

pub struct Diagnostic {
    pub rule_id: String,
    pub severity: Severity, // Error | Warning | Info
    pub path: PathBuf,
    pub message: String,
    pub fix: Option<String>,
}

pub enum Severity { Error, Warning, Info }
```

### 2. `RuleRegistry`

```rust
pub struct RuleRegistry { rules: BTreeMap<&'static str, Box<dyn LintRule>> }
impl RuleRegistry {
    pub fn builtin() -> Self;          // registers every shipped rule
    pub fn get(&self, id: &str) -> Option<&dyn LintRule>;
    pub fn ids(&self) -> Vec<&'static str>;
    pub fn run_all(&self, ctx: &LintContext) -> LintReport;
    pub fn run_one(&self, id: &str, ctx: &LintContext) -> Result<Vec<Diagnostic>, LintError>;
}
```

`builtin()` is the single registration site. Adding a rule without inserting it here means the completeness test fails.

### 3. Required rule set

The registry **must** include at least these named rules (ids are stable; they appear in JSON output and in the issue-driven quality bar):

| Id | Scope | Check |
|---|---|---|
| `V-ROOT` | App | `app.yaml` / `app.json` names a `root_agent` (or `start_agent`) that exists under `agents/`. **This is the #86 closer.** |
| `V001` | App | `app.yaml` or `app.json` exists at the app root |
| `V002` | App | `display_name` is non-empty |
| `V003` | Agent | every `agents/<name>/` directory contains `instruction.txt` or `agent.yaml` |
| `V004` | Tool | every tool referenced by an agent exists under `tools/` |
| `V005` | Tool | tool schema JSON is valid JSON Schema draft 2020-12 |
| `V006` | Evaluation | golden / deterministic eval files reference existing agents (Python V006 analogue) |
| `V-WELCOME` | Deployment | Web Widget deployments declare a `welcome_event` (#397) |
| `V-DEPVER` | Deployment | deployment `app_version` is non-empty and matches `versions/` or a resource name (#397) |
| `V-SCHEMA-*` | * | one rule per key in `schema/app.required.json` |

`schema/app.required.json` is checked in and lists required fields, for example:

```json
{
  "app": ["display_name", "root_agent"],
  "agent": ["instruction"],
  "tool": ["name", "schema"],
  "deployment": ["channel_type"],
  "evaluation": ["display_name"]
}
```

The completeness test loads this file and asserts that for every field `F` in every section there exists a rule whose `id` is `V-SCHEMA-{SECTION}-{FIELD}` or whose description contains that field **and** whose `run` fails when the field is absent. `V-ROOT` satisfies `app.root_agent`.

The remaining rules up to the 60+ floor are structural: YAML parse errors, unknown keys, broken example paths, guardrail references, evaluation dataset paths, environment.json shape (including boolean types from Phase 1 #256), duplicate display names, empty instruction files, and per-resource flags (`--agent`, `--tool`, `--guardrail`) that run a single-resource subset (Python `SINGLE_RESOURCE_RULES`).

### 4. `LintContext` and discovery

```rust
pub struct LintContext {
    pub root: PathBuf,
    pub app: Option<serde_json::Value>,
    pub agents: BTreeMap<String, AgentDoc>,
    pub tools: BTreeMap<String, ToolDoc>,
    pub deployments: BTreeMap<String, DeploymentDoc>,
    pub evaluations: Vec<EvalDoc>,
}

pub fn discover(root: &Path) -> Result<LintContext, LintError>;
```

Discovery is fallible: unreadable roots are `LintError::Io`. A missing `app.yaml` is **not** an I/O error; it is a `V001` diagnostic.

### 5. `LlmLintClient` (`llm` feature)

```rust
pub struct LlmLintClient { http: reqwest::Client, endpoint: Url, api_key_env: &'static str }
impl LlmLintClient {
    pub async fn lint_instructions(&self, files: &[InstructionFile]) -> Result<Vec<Diagnostic>, LintError>;
}
```

Sends instruction text to a Gemini-compatible `generateContent` HTTP endpoint (the `google-genai` analogue). Response JSON is parsed into `Diagnostic` values with `rule_id = "LLM-SEMANTIC"`. No API key is hardcoded; absence of the env var is `LintError::MissingApiKey`. Timeouts are 30s.

The client is a thin wrapper. Prompt text is stored in `crates/cxas-lint/prompts/semantic_review.txt` and asks for JSON only: `[{ "severity", "message", "path" }]`. Non-JSON model output is `LintError::UnparseableModel`.

### 6. Report

```rust
pub struct LintReport { pub diagnostics: Vec<Diagnostic> }
impl LintReport {
    pub fn error_count(&self) -> usize;
    pub fn to_json(&self) -> String;
    pub fn exit_code(&self) -> i32; // 1 if any Error, else 0
}
```

Phase 5 CLI prints `to_json()` by default.

## Data flow

**Structural lint**

1. CLI / library receives `--app-dir`.
2. `discover` reads the tree into `LintContext`.
3. `RuleRegistry::builtin().run_all(&ctx)` runs every rule.
4. `V-ROOT` looks up `root_agent` / `start_agent` on the app doc and requires `agents/{name}` to exist. Missing or dangling root → one `Error` diagnostic with `rule_id = "V-ROOT"`.
5. Report serializes; exit code follows `error_count`.

**Completeness**

1. Test loads `schema/app.required.json`.
2. For each required field, the test builds a fixture that omits only that field.
3. `run_all` must emit at least one `Error` whose rule id is associated with that field (table in `crates/cxas-lint/src/schema_map.rs`).

**LLM lint**

1. Collect `instruction.txt`, `global_instruction.txt`, and callback sources.
2. `LlmLintClient::lint_instructions` POSTs them.
3. Diagnostics merge into a `LintReport` (warnings by default unless the model marks error).

## Error handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum LintError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("unknown rule id {0}")]
    UnknownRule(String),
    #[error("missing API key in env {0}")]
    MissingApiKey(&'static str),
    #[error("model output was not JSON diagnostics")]
    UnparseableModel,
    #[error("gemini http {status}: {body}")]
    Http { status: u16, body: String },
}
```

| Condition | Behavior |
|---|---|
| App dir missing | `LintError::Io` — process-level failure, not a diagnostic |
| App dir present, `app.yaml` missing | `V001` Error diagnostic |
| Root agent missing / dangling | `V-ROOT` Error (#86) |
| Welcome event missing on Web Widget | `V-WELCOME` Error (#397) |
| Deployment version empty | `V-DEPVER` Error (#397) |
| Unknown `--rule` id | `UnknownRule` |
| LLM feature off but `llm-lint` invoked | compile-time: CLI only exposes the subcommand when built with `--features llm` |
| HTTP 401/403 from Gemini | `Http` — non-zero exit, no crash |
| Malformed model JSON | `UnparseableModel` |

Rules themselves do not return `Err` for "check failed"; they push `Diagnostic`. `Err` is for engine failure.

## Testing

1. **#86** — fixture app with agents but no `root_agent` key: `V-ROOT` Error. Fixture with `root_agent: helper` but only `agents/other/`: `V-ROOT` Error. Fixture with `root_agent: main` and `agents/main/instruction.txt`: zero `V-ROOT` diagnostics.
2. **#397** — Web Widget deployment without `welcome_event` fails `V-WELCOME`. Deployment with empty `app_version` fails `V-DEPVER`.
3. **Completeness** — `registry.ids().len() >= 60`. Every field in `schema/app.required.json` has a failing fixture as described above.
4. **Single-resource** — `run_one("V005", ctx)` on a bad tool schema emits `V005` only.
5. **JSON report** — `to_json()` is parseable and contains `rule_id`, `severity`, `path`, `message`.
6. **LLM client** — mock HTTP server returns a JSON array; client maps it to diagnostics. A second test with `{not json}` yields `UnparseableModel`. A third test with no env key yields `MissingApiKey`.
7. **Parity hook** — Phase 0 manifest commands `["lint"]` and `["llm-lint"]` are owned by `cxas-cli` and implemented by calling `cxas-lint`.

No live Gemini calls in default tests. The mock server binds to `127.0.0.1:0`.

## Out of scope

- Wiring `cxas lint` clap flags (Phase 5 calls `cxas_lint::discover` + `run_all`).
- Auto-fix writers (diagnostics may include a `fix` string; applying it is not this phase).
- Hillclimbing or migration integrity checks (Phase 4 may call the registry but does not own it).
