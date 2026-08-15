# Phase 5 Packaging, CLI, Docs, and Release Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the machine-first `cxas` CLI, Actions workflow generation, `diff`/`state`/`deploy`/`trace --raw`, mdBook sidebar structure, and a `deny.toml` dependency policy.

**Architecture:** `cxas_cli::run(argv, transport)` is the testable entry point; `main.rs` is a thin wrapper. Default `--format json` and `--no-input`. Templates write GitHub Actions YAML from `environment.json`.

**Tech Stack:** Rust 2021, `clap` 4, `serde_json`/`serde_yaml`, `assert_cmd`, `predicates`, `cxas-core`, `cxas-evals`, `cxas-lint`, `cxas-migration`, `cxas-state`, `cxas-parity`.

**Spec:** `docs/superpowers/specs/2026-08-15-packaging-cli-ci-design.md`

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

- Modify: `crates/cxas-cli/Cargo.toml`
- Create: `crates/cxas-cli/src/lib.rs`
- Create: `crates/cxas-cli/src/args.rs`
- Create: `crates/cxas-cli/src/output.rs`
- Create: `crates/cxas-cli/src/commands/mod.rs`, `lint.rs`, `pull.rs`, `trace.rs`, `actions.rs`, `deploy.rs`, `diff.rs`, `state.rs`, `evals.rs`, `migrate.rs`
- Modify: `crates/cxas-cli/src/main.rs`
- Create: `crates/cxas-cli/templates/test_workflow.yml.tmpl`
- Create: `crates/cxas-cli/templates/cleanup_workflow.yml.tmpl`
- Create: `book/book.toml`, `book/src/SUMMARY.md`, `book/src/docs.md`, `book/src/examples.md`, `book/src/skills.md`, `book/src/sdk.md`
- Create: `deny.toml`
- Create: `renovate.json`
- Create: `dist-workspace.toml`
- Test: `crates/cxas-cli/tests/json_lint.rs`
- Test: `crates/cxas-cli/tests/actions_init.rs`
- Test: `crates/cxas-cli/tests/trace_raw.rs`
- Test: `crates/cxas-cli/tests/pull_version.rs`
- Test: `crates/cxas-cli/tests/deploy.rs`
- Test: `crates/cxas-cli/tests/diff_state.rs`
- Test: `crates/cxas-cli/tests/parity_cli.rs`
- Test: `crates/cxas-cli/tests/docs_and_deny.rs`

---

### Task 1: Clap parser, JSON envelope, lint command (#55)

**Files:**
- Create: `crates/cxas-cli/src/lib.rs`
- Create: `crates/cxas-cli/src/args.rs`
- Create: `crates/cxas-cli/src/output.rs`
- Create: `crates/cxas-cli/src/commands/mod.rs`
- Create: `crates/cxas-cli/src/commands/lint.rs`
- Modify: `crates/cxas-cli/src/main.rs`
- Test: `crates/cxas-cli/tests/json_lint.rs`

**Interfaces:**
- Consumes: `cxas_lint::{discover, RuleRegistry, LintReport}`
- Produces: `pub fn build_parser() -> clap::Command`, `pub fn run(argv: &[String], out: &mut impl Write) -> i32`, JSON `{ ok, command, data|error }`, default format json

- [ ] **Step 1: Write the failing test**

```rust
use std::fs;
use std::io::Cursor;

#[test]
fn lint_json_is_parseable_and_non_interactive() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("app.yaml"), "display_name: d\n").unwrap();
    let mut buf = Cursor::new(Vec::new());
    let code = cxas_cli::run(
        &[
            "cxas".into(),
            "lint".into(),
            "--app-dir".into(),
            dir.path().display().to_string(),
        ],
        &mut buf,
    );
    let v: serde_json::Value = serde_json::from_slice(buf.get_ref()).unwrap();
    assert!(v["ok"].is_boolean());
    assert_eq!(v["command"], "lint");
    assert_eq!(code, 1, "missing root_agent is an error");
}

#[test]
fn pull_without_location_or_workspace_is_usage() {
    let mut buf = Cursor::new(Vec::new());
    let code = cxas_cli::run(
        &[
            "cxas".into(),
            "pull".into(),
            "--app".into(),
            "demo".into(),
            "--target-dir".into(),
            "/tmp/out".into(),
        ],
        &mut buf,
    );
    assert_eq!(code, 2);
    let v: serde_json::Value = serde_json::from_slice(buf.get_ref()).unwrap();
    assert_eq!(v["error"]["code"], "LOCATION_REQUIRED");
}
```

This task implements `lint` and registers `pull` so the location-required path is testable. `pull --version-id` streaming behavior is Task 3; here `pull` without `--location` and without a workspace file must return `LOCATION_REQUIRED` and exit 2.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-cli lint_json_is_parseable_and_non_interactive --offline`
Expected: FAIL with `cannot find function run`

- [ ] **Step 3: Write minimal implementation**

`build_parser` uses clap with global `--format` defaulting to `json` and `--no-input` defaulting to true. Subcommands include at least `lint` (`--app-dir`) and `pull` (`--app`, `--target-dir`, `--location`, `--version-id`). `run` parses argv (skipping argv0), dispatches, writes one JSON object to `out`, returns exit code. Lint path calls `discover` + `RuleRegistry::builtin().run_all`; `ok` is `error_count()==0`; `data.diagnostics` is the report. Missing location on pull: `{ ok:false, command:"pull", error:{ code:"LOCATION_REQUIRED", message:"location is required and has no default" } }` exit 2.

`main.rs`:

```rust
fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let code = cxas_cli::run(&argv, &mut std::io::stdout());
    std::process::exit(code);
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-cli --offline`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cxas-cli
git commit -m "feat(cli): machine-first JSON lint command (#55)"
```

---

### Task 2: `actions init` matrix from `environment.json` (#46, #54)

**Files:**
- Create: `crates/cxas-cli/src/commands/actions.rs`
- Create: `crates/cxas-cli/templates/test_workflow.yml.tmpl`
- Create: `crates/cxas-cli/templates/cleanup_workflow.yml.tmpl`
- Test: `crates/cxas-cli/tests/actions_init.rs`

**Interfaces:**
- Consumes: `--app-dir`, optional `environment.json`
- Produces: `.github/workflows/test_<agent>.yml` with a job matrix; `init-github-action` alias

- [ ] **Step 1: Write the failing test**

```rust
use std::fs;
use std::io::Cursor;

#[test]
fn actions_init_writes_matrix_for_each_environment() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("app.yaml"), "display_name: pilot\nroot_agent: main\n").unwrap();
    fs::write(dir.path().join("environment.json"), r#"{"dev":{},"prod":{}}"#).unwrap();
    let mut buf = Cursor::new(Vec::new());
    let code = cxas_cli::run(
        &[
            "cxas".into(),
            "actions".into(),
            "init".into(),
            "--app-dir".into(),
            dir.path().display().to_string(),
        ],
        &mut buf,
    );
    assert_eq!(code, 0);
    let wf = fs::read_to_string(dir.path().join(".github/workflows/test_pilot.yml")).unwrap();
    assert!(wf.contains("dev"));
    assert!(wf.contains("prod"));
    assert!(wf.contains("cxas lint"));
}

#[test]
fn init_github_action_alias_writes_the_same_workflow() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("app.yaml"), "display_name: pilot\nroot_agent: main\n").unwrap();
    let mut buf = Cursor::new(Vec::new());
    let code = cxas_cli::run(
        &[
            "cxas".into(),
            "init-github-action".into(),
            "--app-dir".into(),
            dir.path().display().to_string(),
        ],
        &mut buf,
    );
    assert_eq!(code, 0);
    assert!(dir.path().join(".github/workflows/test_pilot.yml").exists());
}

#[test]
fn auto_create_wif_is_manual() {
    let mut buf = Cursor::new(Vec::new());
    let code = cxas_cli::run(
        &[
            "cxas".into(),
            "actions".into(),
            "init".into(),
            "--app-dir".into(),
            ".".into(),
            "--auto-create-wif".into(),
        ],
        &mut buf,
    );
    assert_eq!(code, 2);
    let v: serde_json::Value = serde_json::from_slice(buf.get_ref()).unwrap();
    assert_eq!(v["error"]["code"], "WIF_MANUAL");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-cli actions_init_writes_matrix_for_each_environment --offline`
Expected: FAIL (unknown subcommand `actions` or missing workflow file)

- [ ] **Step 3: Write minimal implementation**

Parse `environment.json` object keys (default matrix `["default"]` if the file is absent). Write YAML containing:

```yaml
on: [pull_request]
jobs:
  test:
    strategy:
      matrix:
        environment: [dev, prod]
    steps:
      - run: cxas lint --app-dir . --format json
      - run: cxas run --wait --format json
```

Agent name is `display_name` from `app.yaml`, sanitized to `[a-z0-9_]+`. `--auto-create-wif` short-circuits with `WIF_MANUAL`. Register both `actions init` and `init-github-action`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-cli --offline`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cxas-cli
git commit -m "feat(cli): generate multi-environment GitHub Actions workflows (#46, #54)"
```

---

### Task 3: `pull --version-id`, `trace --raw`, `evals report` turns (#252, #350, #206, #298)

**Files:**
- Create: `crates/cxas-cli/src/commands/pull.rs`
- Create: `crates/cxas-cli/src/commands/trace.rs`
- Create: `crates/cxas-cli/src/commands/evals.rs`
- Test: `crates/cxas-cli/tests/pull_version.rs`
- Test: `crates/cxas-cli/tests/trace_raw.rs`

**Interfaces:**
- Consumes: `Apps::export_app_version`, `EvalReport`, a test `MockTransport` registered via `cxas_cli::set_transport_for_test`
- Produces: extracted app dir; NDJSON trace lines with `raw`; report JSON with `turns`

- [ ] **Step 1: Write the failing test**

```rust
// crates/cxas-cli/tests/pull_version.rs
use std::io::Cursor;

#[tokio::test]
async fn pull_forwards_version_id_to_transport() {
    let rec = cxas_cli::test_support::RecordingTransport::default();
    cxas_cli::set_transport_for_test(rec.clone());
    rec.stub_export(vec![0u8; 5 * 1024 * 1024]);
    let dir = tempfile::tempdir().unwrap();
    let mut buf = Cursor::new(Vec::new());
    let code = cxas_cli::run(
        &[
            "cxas".into(),
            "pull".into(),
            "--app".into(),
            "projects/p/locations/us/apps/a".into(),
            "--location".into(),
            "us".into(),
            "--target-dir".into(),
            dir.path().display().to_string(),
            "--version-id".into(),
            "v3".into(),
        ],
        &mut buf,
    );
    assert_eq!(code, 0);
    assert_eq!(rec.last_export_version().as_deref(), Some("v3"));
    assert!(rec.last_export_bytes() >= 5 * 1024 * 1024);
}
```

```rust
// crates/cxas-cli/tests/trace_raw.rs
use std::io::Cursor;

#[test]
fn trace_raw_emits_one_json_object_per_turn_with_raw() {
    cxas_cli::test_support::script_trace(vec![
        serde_json::json!({"text": "hi"}),
        serde_json::json!({"text": "yo"}),
    ]);
    let mut buf = Cursor::new(Vec::new());
    let code = cxas_cli::run(
        &[
            "cxas".into(),
            "trace".into(),
            "--app-name".into(),
            "projects/p/locations/us/apps/a".into(),
            "--location".into(),
            "us".into(),
            "--raw".into(),
        ],
        &mut buf,
    );
    assert_eq!(code, 0);
    let text = String::from_utf8(buf.into_inner()).unwrap();
    let lines: Vec<&str> = text.lines().filter(|l| l.starts_with('{') && l.contains("turn")).collect();
    // When --format json wraps a single object, the data field is an array of turns.
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    let turns = v["data"]["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 2);
    assert!(turns[0]["raw"].is_object());
    assert!(turns[1]["raw"].is_object());
}

#[test]
fn evals_report_json_includes_turns() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("sim_results.json"), r#"{"turns":[{"turn_index":0}]}"#).unwrap();
    let mut buf = Cursor::new(Vec::new());
    let out = dir.path().join("out.json");
    let code = cxas_cli::run(
        &[
            "cxas".into(),
            "evals".into(),
            "report".into(),
            "--output-dir".into(),
            dir.path().display().to_string(),
            "--format".into(),
            "json".into(),
            "--output".into(),
            out.display().to_string(),
        ],
        &mut buf,
    );
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(out).unwrap()).unwrap();
    assert!(v["turns"].is_array());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-cli pull_forwards_version_id_to_transport --offline`
Expected: FAIL (`set_transport_for_test` missing or version not forwarded)

- [ ] **Step 3: Write minimal implementation**

Thread a process-level `OnceLock<Arc<dyn CesTransport>>` for tests. `pull` calls `export_app_version` when `--version-id` is set. `trace --raw` fills `data.turns[i].raw` from the scripted proto JSON. `evals report` reads `sim_results.json` (or calls `generate_combined_json_report` on a constructed `EvalReport`) and writes `--output`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-cli --offline`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cxas-cli
git commit -m "feat(cli): versioned pull, raw traces, and turn-level eval reports (#252, #350, #206)"
```

---

### Task 4: `deploy`, `diff`, `state` (#386, #131, #270)

**Files:**
- Create: `crates/cxas-cli/src/commands/deploy.rs`
- Create: `crates/cxas-cli/src/commands/diff.rs`
- Create: `crates/cxas-cli/src/commands/state.rs`
- Test: `crates/cxas-cli/tests/deploy.rs`
- Test: `crates/cxas-cli/tests/diff_state.rs`

**Interfaces:**
- Consumes: `Apps::import_app`, `Versions::create_version`, `Deployments::create_deployment`, `resolve_workspace`, `hash_app_dir`, `diff_trees`
- Produces: `cxas deploy`, `cxas diff` (exit 1 on drift), `cxas state` JSON hash + profile

- [ ] **Step 1: Write the failing test**

```rust
// crates/cxas-cli/tests/deploy.rs
use std::io::Cursor;

#[test]
fn deploy_calls_import_version_and_deployment() {
    let rec = cxas_cli::test_support::RecordingTransport::default();
    cxas_cli::set_transport_for_test(rec.clone());
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("app.yaml"), "display_name: d\nroot_agent: main\n").unwrap();
    let mut buf = Cursor::new(Vec::new());
    let code = cxas_cli::run(
        &[
            "cxas".into(),
            "deploy".into(),
            "--app-dir".into(),
            dir.path().display().to_string(),
            "--project-id".into(),
            "p".into(),
            "--location".into(),
            "us".into(),
            "--channel-type".into(),
            "API".into(),
        ],
        &mut buf,
    );
    assert_eq!(code, 0);
    assert!(rec.imported());
    assert!(rec.version_created());
    assert!(rec.deployment_created());
}
```

```rust
// crates/cxas-cli/tests/diff_state.rs
use std::fs;
use std::io::Cursor;

#[test]
fn state_prints_hash_and_location() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("cxas.workspace.yaml"),
        "profiles:\n  x:\n    project_id: p\n    location: europe-west1\nactive: x\n",
    )
    .unwrap();
    fs::write(dir.path().join("app.yaml"), "display_name: d\n").unwrap();
    let mut buf = Cursor::new(Vec::new());
    let code = cxas_cli::run(
        &[
            "cxas".into(),
            "state".into(),
            "--app-dir".into(),
            dir.path().display().to_string(),
        ],
        &mut buf,
    );
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_slice(buf.get_ref()).unwrap();
    assert!(v["data"]["hash"].as_str().unwrap().len() == 64);
    assert_eq!(v["data"]["profile"]["location"], "europe-west1");
}

#[test]
fn diff_exits_one_on_drift() {
    let rec = cxas_cli::test_support::RecordingTransport::default();
    rec.stub_remote_tree(&[("tools/only-remote.yaml", "x")]);
    cxas_cli::set_transport_for_test(rec);
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("app.yaml"), "display_name: d\n").unwrap();
    let mut buf = Cursor::new(Vec::new());
    let code = cxas_cli::run(
        &[
            "cxas".into(),
            "diff".into(),
            "--app-dir".into(),
            dir.path().display().to_string(),
            "--location".into(),
            "us".into(),
            "--app".into(),
            "projects/p/locations/us/apps/a".into(),
        ],
        &mut buf,
    );
    assert_eq!(code, 1);
    let v: serde_json::Value = serde_json::from_slice(buf.get_ref()).unwrap();
    assert_eq!(v["error"]["code"], "DRIFT");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-cli deploy_calls_import_version_and_deployment --offline`
Expected: FAIL (unknown `deploy` or flags not recorded)

- [ ] **Step 3: Write minimal implementation**

`deploy` sequences mockable `import_app` → `create_version` → `create_deployment`. `state` calls `resolve_workspace` (falling back to `--location` / `--project-id` flags) then `hash_app_dir`. `diff` hashes local, asks the transport for a remote `AppTree`, and exits 1 when `diff_trees` is non-empty unless `--allow-drift`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-cli --offline`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cxas-cli
git commit -m "feat(cli): deploy, state, and diff commands (#386, #131, #270)"
```

---

### Task 5: Parity walk, mdBook sidebar, deny.toml (#99)

**Files:**
- Modify: `crates/cxas-cli/src/args.rs` (register every Phase 0 argv as a clap command, even if the body is a `FEATURE_DISABLED` or `NOT_IMPLEMENTED` JSON — **not** a placeholder comment: each handler returns a typed `{ ok:false, error:{ code:"NOT_IMPLEMENTED", message:"<command> lands in this crate after its owning phase" } }` only for commands whose crate is still a stub; after Phases 1–4 those owners exist, so implement thin delegates: `migrate dfcx` → `MigrationPipeline::run`, `run-session` → TTY_REQUIRED without a TTY, etc.)
- Create: `book/book.toml`
- Create: `book/src/SUMMARY.md`
- Create: `book/src/docs.md`, `book/src/examples.md`, `book/src/skills.md`, `book/src/sdk.md`
- Create: `deny.toml`
- Create: `renovate.json`
- Create: `dist-workspace.toml`
- Test: `crates/cxas-cli/tests/parity_cli.rs`
- Test: `crates/cxas-cli/tests/docs_and_deny.rs`

**Interfaces:**
- Consumes: `cxas_parity::load_bundled`
- Produces: every non-extension CLI argv findable on `build_parser()`; mdBook four-section SUMMARY; `deny.toml` with `licenses` and `advisories`

- [ ] **Step 1: Write the failing test**

```rust
// crates/cxas-cli/tests/parity_cli.rs
#[test]
fn every_parity_command_is_a_clap_subcommand() {
    let manifest = cxas_parity::load_bundled().unwrap();
    let parser = cxas_cli::build_parser();
    for cmd in manifest.commands_for_crate("cxas-cli") {
        let mut current = &parser;
        for (i, part) in cmd.argv.iter().enumerate() {
            current = current
                .find_subcommand(part)
                .unwrap_or_else(|| panic!("missing clap path {:?} at {part}", cmd.argv));
            if i + 1 == cmd.argv.len() {
                break;
            }
        }
    }
}
```

```rust
// crates/cxas-cli/tests/docs_and_deny.rs
#[test]
fn mdbook_summary_has_required_sections() {
    let summary = include_str!("../../../book/src/SUMMARY.md");
    for needle in ["Docs", "Examples", "Agent Skills", "Core SDK"] {
        assert!(summary.contains(needle), "SUMMARY.md missing {needle}");
    }
}

#[test]
fn deny_toml_has_licenses_and_advisories() {
    let text = include_str!("../../../deny.toml");
    assert!(text.contains("[licenses]"));
    assert!(text.contains("[advisories]"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-cli every_parity_command_is_a_clap_subcommand --offline`
Expected: FAIL on the first argv not yet registered (e.g. `conversations`)

- [ ] **Step 3: Write minimal implementation**

Register every argv from the Phase 0 table on the clap `Command`. Handlers that are not yet deep-implemented still run and return JSON `NOT_IMPLEMENTED` with exit 1 (this is a real, stable contract — not an empty stub comment). Implement `migrate dfcx` by mapping flags onto `MigrationPipeline`. Write:

`book/src/SUMMARY.md`:

```markdown
# Summary

- [Docs](docs.md)
- [Examples](examples.md)
- [Agent Skills](skills.md)
- [Core SDK](sdk.md)
```

`deny.toml`:

```toml
[licenses]
allow = ["Apache-2.0", "MIT", "BSD-3-Clause", "ISC", "Unicode-3.0"]

[advisories]
yanked = "deny"
```

`renovate.json`: `{ "extends": ["config:recommended"] }`
`dist-workspace.toml`: `[workspace] members = ["crates/cxas-cli"]` plus a `[dist]` table naming the five targets from the spec.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-cli --offline`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cxas-cli book deny.toml renovate.json dist-workspace.toml
git commit -m "feat(cli): full parity command map, mdBook sidebar, and cargo-deny policy (#99)"
```
