# Phase 3 Lint and Validation Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `cxas-lint` as a `LintRule` registry with a completeness test against `schema/app.required.json`, closing missing root-agent validation and Web Widget deployment checks.

**Architecture:** Pure rules over a discovered `LintContext`. `V-ROOT` is a first-class rule. `LlmLintClient` is feature-gated (`llm`) and talks to an injected HTTP endpoint.

**Tech Stack:** Rust 2021, `serde`/`serde_json`/`serde_yaml`, `thiserror`, `walkdir`, `reqwest` (optional `llm` feature), `tokio` (llm tests), `httpmock` or a tiny `std::net` listener for HTTP tests.

**Spec:** `docs/superpowers/specs/2026-08-15-lint-validation-design.md`

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

- Modify: `crates/cxas-lint/Cargo.toml`, `src/lib.rs`
- Create: `schema/app.required.json`
- Create: `crates/cxas-lint/src/error.rs`, `diagnostic.rs`, `context.rs`, `registry.rs`
- Create: `crates/cxas-lint/src/rules/mod.rs`, `root.rs`, `schema.rs`, `welcome.rs`, `depver.rs`, plus one file per additional structural rule (or a `structural.rs` that registers many small structs)
- Create: `crates/cxas-lint/src/schema_map.rs`
- Create: `crates/cxas-lint/src/llm.rs` (behind `llm`)
- Create: `crates/cxas-lint/prompts/semantic_review.txt`
- Test: `crates/cxas-lint/tests/root_agent.rs`
- Test: `crates/cxas-lint/tests/completeness.rs`
- Test: `crates/cxas-lint/tests/welcome.rs`
- Test: `crates/cxas-lint/tests/llm_client.rs`

---

### Task 1: Discovery, diagnostics, and `V-ROOT` (#86)

**Files:**
- Create: `crates/cxas-lint/src/error.rs`
- Create: `crates/cxas-lint/src/diagnostic.rs`
- Create: `crates/cxas-lint/src/context.rs`
- Create: `crates/cxas-lint/src/registry.rs`
- Create: `crates/cxas-lint/src/rules/mod.rs`
- Create: `crates/cxas-lint/src/rules/root.rs`
- Test: `crates/cxas-lint/tests/root_agent.rs`

**Interfaces:**
- Consumes: an app directory on disk
- Produces: `discover`, `LintContext`, `LintRule`, `RuleRegistry`, `VRootRule` with `id() == "V-ROOT"`, `LintReport`

- [ ] **Step 1: Write the failing test**

```rust
use cxas_lint::{discover, RuleRegistry};
use std::fs;

fn fixture(files: &[(&str, &str)]) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    for (rel, body) in files {
        let p = tmp.path().join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }
    tmp
}

#[test]
fn missing_root_agent_is_v_root_error() {
    let dir = fixture(&[("app.yaml", "display_name: demo\n"), ("agents/main/instruction.txt", "hi")]);
    let ctx = discover(dir.path()).unwrap();
    let report = RuleRegistry::builtin().run_all(&ctx);
    assert!(report.diagnostics.iter().any(|d| d.rule_id == "V-ROOT"));
    assert!(report.error_count() >= 1);
}

#[test]
fn dangling_root_agent_is_v_root_error() {
    let dir = fixture(&[
        ("app.yaml", "display_name: demo\nroot_agent: helper\n"),
        ("agents/other/instruction.txt", "x"),
    ]);
    let ctx = discover(dir.path()).unwrap();
    let report = RuleRegistry::builtin().run_all(&ctx);
    assert!(report.diagnostics.iter().any(|d| d.rule_id == "V-ROOT"));
}

#[test]
fn valid_root_agent_is_silent() {
    let dir = fixture(&[
        ("app.yaml", "display_name: demo\nroot_agent: main\n"),
        ("agents/main/instruction.txt", "you are main"),
    ]);
    let ctx = discover(dir.path()).unwrap();
    let report = RuleRegistry::builtin().run_all(&ctx);
    assert!(!report.diagnostics.iter().any(|d| d.rule_id == "V-ROOT"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-lint missing_root_agent_is_v_root_error --offline`
Expected: FAIL with `cannot find function discover` or `cannot find struct RuleRegistry`

- [ ] **Step 3: Write minimal implementation**

`discover` reads `app.yaml`/`app.json` into `serde_json::Value`, lists `agents/*` directories. `VRootRule::run` looks up `root_agent` or `start_agent`; missing or not in `ctx.agents` → one `Diagnostic { rule_id: "V-ROOT", severity: Error, ... }`. `RuleRegistry::builtin` inserts `V-ROOT`. `LintReport::error_count` counts `Severity::Error`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-lint --offline`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cxas-lint
git commit -m "feat(lint): validate root_agent exists (#86)"
```

---

### Task 2: Schema completeness floor (60+ rules)

**Files:**
- Create: `schema/app.required.json`
- Create: `crates/cxas-lint/src/schema_map.rs`
- Create: `crates/cxas-lint/src/rules/schema.rs`
- Create: additional rule structs so `ids().len() >= 60`
- Test: `crates/cxas-lint/tests/completeness.rs`

**Interfaces:**
- Consumes: `schema/app.required.json`, `RuleRegistry::builtin`
- Produces: `V-SCHEMA-{SECTION}-{FIELD}` rules, `schema_map::FIELD_RULES: &[(&str, &str, &str)]` as `(section, field, rule_id)`

- [ ] **Step 1: Write the failing test**

```rust
use cxas_lint::{discover, RuleRegistry, schema_map};

#[test]
fn registry_has_at_least_sixty_rules() {
    assert!(RuleRegistry::builtin().ids().len() >= 60);
}

#[test]
fn every_required_field_has_a_failing_fixture() {
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../../../schema/app.required.json")).unwrap();
    for (section, fields) in schema.as_object().unwrap() {
        for field in fields.as_array().unwrap() {
            let field = field.as_str().unwrap();
            let rule_id = schema_map::rule_id_for(section, field)
                .unwrap_or_else(|| panic!("no rule mapped for {section}.{field}"));
            let dir = schema_map::fixture_omitting(section, field);
            let ctx = discover(dir.path()).unwrap();
            let report = RuleRegistry::builtin().run_all(&ctx);
            assert!(
                report.diagnostics.iter().any(|d| d.rule_id == rule_id && d.severity == cxas_lint::Severity::Error),
                "{section}.{field} should trigger {rule_id}"
            );
        }
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-lint registry_has_at_least_sixty_rules --offline`
Expected: FAIL with assertion `ids().len() >= 60` (only `V-ROOT` exists)

- [ ] **Step 3: Write minimal implementation**

`schema/app.required.json`:

```json
{
  "app": ["display_name", "root_agent"],
  "agent": ["instruction"],
  "tool": ["name", "schema"],
  "deployment": ["channel_type"],
  "evaluation": ["display_name"]
}
```

`rule_id_for("app", "root_agent")` returns `Some("V-ROOT")`. Other fields map to `V-SCHEMA-APP-DISPLAY_NAME` etc. Implement each as a struct that errors when the field is absent. Add numbered structural rules `V010`… until `ids().len() >= 60` (each can check a distinct unused key or a distinct file-name pattern; they must still be real `LintRule` impls with unique ids — a macro `define_presence_rule!(V010, "agents/{name}/examples.yaml is optional-info")` that emits `Info` on missing optional files is acceptable provided each id is unique and `run` executes).

`fixture_omitting` writes a complete valid app then deletes the one field.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-lint --offline`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add schema/app.required.json crates/cxas-lint
git commit -m "feat(lint): rule registry completeness against required schema fields"
```

---

### Task 3: Web Widget welcome event and deployment version (#397)

**Files:**
- Create: `crates/cxas-lint/src/rules/welcome.rs`
- Create: `crates/cxas-lint/src/rules/depver.rs`
- Test: `crates/cxas-lint/tests/welcome.rs`

**Interfaces:**
- Consumes: `LintContext.deployments`
- Produces: rules `V-WELCOME`, `V-DEPVER`

- [ ] **Step 1: Write the failing test**

```rust
use cxas_lint::{discover, RuleRegistry};
use std::fs;

#[test]
fn web_widget_without_welcome_event_fails() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("app.yaml"), "display_name: d\nroot_agent: main\n").unwrap();
    fs::create_dir_all(tmp.path().join("agents/main")).unwrap();
    fs::write(tmp.path().join("agents/main/instruction.txt"), "x").unwrap();
    fs::create_dir_all(tmp.path().join("deployments/web")).unwrap();
    fs::write(
        tmp.path().join("deployments/web/deployment.yaml"),
        "channel_type: WEB_WIDGET\napp_version: v1\n",
    )
    .unwrap();
    let report = RuleRegistry::builtin().run_all(&discover(tmp.path()).unwrap());
    assert!(report.diagnostics.iter().any(|d| d.rule_id == "V-WELCOME"));
}

#[test]
fn empty_app_version_fails_depver() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("app.yaml"), "display_name: d\nroot_agent: main\n").unwrap();
    fs::create_dir_all(tmp.path().join("agents/main")).unwrap();
    fs::write(tmp.path().join("agents/main/instruction.txt"), "x").unwrap();
    fs::create_dir_all(tmp.path().join("deployments/api")).unwrap();
    fs::write(
        tmp.path().join("deployments/api/deployment.yaml"),
        "channel_type: API\napp_version: \"\"\n",
    )
    .unwrap();
    let report = RuleRegistry::builtin().run_all(&discover(tmp.path()).unwrap());
    assert!(report.diagnostics.iter().any(|d| d.rule_id == "V-DEPVER"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-lint web_widget_without_welcome_event_fails --offline`
Expected: FAIL (no `V-WELCOME` diagnostic)

- [ ] **Step 3: Write minimal implementation**

Discovery loads `deployments/*/deployment.yaml`. `V-WELCOME` fires when `channel_type == WEB_WIDGET` and `welcome_event` is missing/empty. `V-DEPVER` fires when `app_version` is missing/empty. Register both in `builtin()`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-lint --offline`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cxas-lint
git commit -m "feat(lint): welcome-event and deployment-version rules (#397)"
```

---

### Task 4: JSON report and `LlmLintClient` (`llm` feature)

**Files:**
- Modify: `crates/cxas-lint/src/diagnostic.rs` (`LintReport::to_json`, `exit_code`)
- Create: `crates/cxas-lint/src/llm.rs`
- Create: `crates/cxas-lint/prompts/semantic_review.txt`
- Modify: `crates/cxas-lint/Cargo.toml` (`[features] llm = ["reqwest","tokio"]`)
- Test: `crates/cxas-lint/tests/llm_client.rs`

**Interfaces:**
- Consumes: instruction files, HTTP endpoint
- Produces: `LintReport::to_json`, `LlmLintClient::lint_instructions`, `LintError::{MissingApiKey,UnparseableModel,Http}`

- [ ] **Step 1: Write the failing test**

```rust
use cxas_lint::{LintReport, Diagnostic, Severity};
use std::path::PathBuf;

#[test]
fn json_report_contains_stable_fields() {
    let report = LintReport {
        diagnostics: vec![Diagnostic {
            rule_id: "V-ROOT".into(),
            severity: Severity::Error,
            path: PathBuf::from("app.yaml"),
            message: "missing root_agent".into(),
            fix: None,
        }],
    };
    let v: serde_json::Value = serde_json::from_str(&report.to_json()).unwrap();
    assert_eq!(v["diagnostics"][0]["rule_id"], "V-ROOT");
    assert_eq!(report.exit_code(), 1);
}

#[tokio::test]
async fn llm_client_maps_json_array() {
    // Start a one-shot hyper listener on 127.0.0.1:0 that returns
    // [{"severity":"warning","message":"vague","path":"instruction.txt"}]
    let (url, _join) = cxas_lint::test_support::spawn_json_listener(
        r#"[{"severity":"warning","message":"vague","path":"instruction.txt"}]"#,
    )
    .await;
    std::env::set_var("CXAS_GEMINI_API_KEY", "test");
    let client = cxas_lint::LlmLintClient::new(&url);
    let diags = client
        .lint_instructions(&[cxas_lint::InstructionFile {
            path: PathBuf::from("instruction.txt"),
            body: "be nice".into(),
        }])
        .await
        .unwrap();
    assert_eq!(diags[0].rule_id, "LLM-SEMANTIC");
    assert_eq!(diags[0].message, "vague");
}

#[tokio::test]
async fn llm_client_rejects_non_json() {
    let (url, _join) = cxas_lint::test_support::spawn_json_listener("not json").await;
    std::env::set_var("CXAS_GEMINI_API_KEY", "test");
    let err = cxas_lint::LlmLintClient::new(&url)
        .lint_instructions(&[])
        .await
        .unwrap_err();
    assert!(matches!(err, cxas_lint::LintError::UnparseableModel));
}

#[tokio::test]
async fn llm_client_requires_api_key() {
    std::env::remove_var("CXAS_GEMINI_API_KEY");
    let err = cxas_lint::LlmLintClient::new("http://127.0.0.1:1")
        .lint_instructions(&[])
        .await
        .unwrap_err();
    assert!(matches!(err, cxas_lint::LintError::MissingApiKey(_)));
}
```

Gate the three `llm_client_*` tests with `#[cfg(feature = "llm")]`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-lint json_report_contains_stable_fields --offline`
Expected: FAIL with `no method named to_json`

- [ ] **Step 3: Write minimal implementation**

`to_json` serde-serializes the report. `exit_code` is 1 if `error_count() > 0` else 0. `LlmLintClient::lint_instructions` returns `MissingApiKey` when `CXAS_GEMINI_API_KEY` is unset; otherwise POSTs the prompt + files, parses a JSON array, maps each object to `Diagnostic { rule_id: "LLM-SEMANTIC", ... }`. Non-array JSON → `UnparseableModel`. `test_support::spawn_json_listener` binds `TcpListener::bind("127.0.0.1:0")` and writes a minimal HTTP/1.1 response (keep this helper under `#[cfg(test)]`).

Prompt file `semantic_review.txt`:

```
Return only a JSON array of objects with keys severity, message, path.
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-lint --offline`
Run: `cargo test -p cxas-lint --features llm --offline`
Expected: PASS both

- [ ] **Step 5: Commit**

```bash
git add crates/cxas-lint
git commit -m "feat(lint): JSON reports and feature-gated Gemini llm-lint client"
```
