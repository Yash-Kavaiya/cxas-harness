# Phase 5 — Packaging, CLI, Docs, and Release Design

> **Withdrawn 2026-08-16.** `dist-workspace.toml` and `renovate.json` were
> removed from the repository. The `cargo-dist` target list configured a release
> job that does not exist here, and the Renovate stub is inert unless the GitHub
> app is installed on the repository, which it is not. The prose below is left as
> the dated record of what this phase decided; the packaging requirement itself
> remains open and is listed on `docs-site/limits.html`. See the withdrawn-artifacts
> table in `docs/superpowers/coverage-map.md`.


**Date:** 2026-08-15
**Status:** Approved from the product briefs (retired 2026-08-16; requirements restated here)
**Product:** `cxas-harness`
**Phase:** 5 of 5 — packaging, docs, and release
**Depends on:** Phases 0–4 crates (`cxas-core`, `cxas-evals`, `cxas-lint`, `cxas-migration`, `cxas-state`, `cxas-parity`)

## Purpose

Ship the `cxas` binary as a machine-first CLI, statically linked per platform, with native GitHub Actions generation, content-addressed `diff`/`state`, lossless `trace` JSON, a `deploy` command, versioned `pull`, mdBook docs that preserve the Python site's Docs / Examples / Agent Skills / Core SDK link structure, and a dependency-policy gate.

**Issue-driven quality bar:** this phase closes **#55** (CLI ergonomics for coding agents and automation), **#54** (multi-environment GitHub Actions via `environment.json`), **#46** (formal GitHub Actions), **#350** (`cxas trace` per-turn raw JSON), **#386** (native `cxas deploy`), **#252** (CLI flag `pull --version-id`, using Phase 1 `export_app_version`), **#206** (CLI `evals report` includes turn rows), and **#99** (Dependency Dashboard — `cargo-deny` / `Renovate` equivalents so the Rust graph is reviewable). Additive commands `diff`, `state`, and `actions init` implement **#131** / **#270** at the CLI layer. `cxas-scrapi` parity means every Phase 0 CLI argv has a clap subcommand; exit codes and JSON schemas are stable enough for agents to parse.

## Architecture

```
cxas (clap)
  --format json|human     (default json)
  --no-input              (default true; flag kept for Python parity)
  --oauth-token
        |
        +-- pull/push/lint/run/...     --> existing crates
        +-- actions init               --> workflow templates
        +-- diff / state               --> cxas-state
        +-- deploy                     --> cxas-core Deployments
        +-- trace                      --> per-turn raw JSON
        |
cargo-dist / cross  -->  platform binaries
mdBook              -->  book/ (Docs, Examples, Agent Skills, Core SDK)
deny.toml           -->  cargo-deny (#99)
```

The binary is the only default install artifact. Agent Skills that Python copied as `.agents` / `.claude` / `.gemini` directories ship as versioned files under `share/cxas/skills/` inside the release tarball, not as unversioned hatch `shared-data`.

## Components

### 1. `cxas-cli` binary

Clap structure matches the Phase 0 argv table plus extensions:

| Subcommand | Behavior | Exit 0 | Exit 1 | Exit 2 |
|---|---|---|---|---|
| `pull` | export app to dir; `--version-id` optional (#252) | success | CES/IO error | usage |
| `push` | import dir | success | CES/IO | usage |
| `lint` | `cxas_lint::run_all`; stdout JSON | no Error diags | Error diags or engine fail | usage |
| `llm-lint` | requires `--features llm` | no Error diags | engine/HTTP | usage |
| `run` | trigger eval; `--wait` polls | eval PASS | eval FAIL | usage |
| `evals report` | `EvalReport` HTML or JSON; includes `turns` (#206) | wrote file | fail | usage |
| `trace` | session trace; `--raw` emits per-turn JSON (#350) | success | fail | usage |
| `actions init` | write `.github/workflows/*` (#46, #54) | wrote files | fail | usage |
| `init-github-action` | Python-compatible alias of `actions init` | same | same | same |
| `deploy` | create/update deployment from a local Python or Rust app dir (#386) | success | fail | usage |
| `diff` | `cxas-state` local vs remote (#131) | no drift | drift (still valid) | usage |
| `state` | print `StateHash` hex + profile (#270) | success | fail | usage |
| `migrate dfcx` | Phase 4 pipeline; default `--yes` | success | fail | usage |
| remaining Phase 0 commands | delegate to the owning crate | success | fail | usage |

Global `--format json` (default) wraps successes as:

```json
{ "ok": true, "command": "pull", "data": { } }
```

and failures as:

```json
{ "ok": false, "command": "pull", "error": { "code": "CES_NOT_FOUND", "message": "..." } }
```

`--format human` prints the Python-like lines. Coding agents (#55) are expected to use the default JSON.

`--no-input` defaults **on**. There is no hidden interactive prompt in the default feature set. `run-session` in non-TTY with `--no-input` exits 2 with `error.code = "TTY_REQUIRED"`.

### 2. `cxas pull --version-id` (#252)

```
cxas pull --app projects/p/locations/us/apps/a --target-dir ./out --version-id v3
```

Calls `Apps::export_app_version`. Without `--version-id`, calls `export_app` (latest). Both use the Phase 1 streamed `ExportHandle` so ≥ 4 MB apps succeed (#298 exercised at the CLI with a mock transport).

### 3. `cxas trace` raw JSON (#350)

Each turn is one JSON object on stdout (newline-delimited) when `--raw` is set:

```json
{ "turn": 0, "user": {...}, "agent": {...}, "raw": { /* proto JSON */ } }
```

The `raw` field is the untruncated proto JSON of the CES turn. Without `--raw`, a compact object omits `raw` (Python's lossy default). `cxas-harness` default for agents is `--raw` when `--format json`.

### 4. `cxas actions init` / `init-github-action` (#46, #54)

Writes:

- `.github/workflows/test_<agent>.yml` — lint, test-tools, `cxas run --wait --format json`
- `.github/workflows/cleanup_<agent>.yml` unless `--no-cleanup`
- Matrix jobs when `environment.json` lists multiple environments (keys become matrix entries) (#54)

The workflow authenticates via Workload Identity Federation flags (`--workload-identity-provider`, `--service-account`) when supplied; otherwise it documents the required `secrets`. `--auto-create-wif` is **not** implemented in v1 (it mutates live GCP); the flag exists, prints JSON `{ "ok": false, "error": { "code": "WIF_MANUAL" } }`, and exits 2 so agents do not hang.

### 5. `cxas deploy` (#386)

Deploys a local multi-file app directory (Python SDK app or a pulled CXAS tree) by `push` + `create_version` + `Deployments::create_deployment` / `update_deployment`. Accepts `--channel-type` and `--noise-cancellation` (Phase 1 #403). Requires `Location` via `--location` or `cxas-state` workspace resolution — never defaults to `"global"`.

### 6. `cxas diff` / `cxas state` (#131, #270)

`state` prints `{ "hash": "<hex>", "profile": { "name", "project_id", "location" } }` from `resolve_workspace` + `hash_app_dir`.

`diff` pulls a remote tree into memory (mockable) and prints `StateDiff`. Exit 1 on any added/removed/changed path so CI can gate on drift; `--allow-drift` forces exit 0.

### 7. Packaging

- `cargo-dist` (or `cross`) produces one statically linked binary per target: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`.
- Default features: no `sheets`, `bigquery`, `audio`, `tui`, `llm`. Release notes document `--features` for those.
- `dist-workspace.toml` at repo root; CI release job uploads artifacts.
- Binary size budget for the default linux-gnu build: fail CI if the stripped binary exceeds 40 MiB (guards against accidentally enabling heavy features).

### 8. Docs

`book/` is an mdBook. Sidebar sections, in order: **Docs**, **Examples**, **Agent Skills**, **Core SDK**. This preserves the Python MkDocs Material link structure from the `cxas-scrapi` README. Content may start as stubs that link to the Superpowers specs; the structure itself is required.

### 9. Dependency dashboard (#99)

- `deny.toml` with `cargo-deny` checks: licenses (Apache-2.0 / MIT / BSD / ISC / Unicode allowed), bans on the Python-bloat analogues if they appear as crates (`polars` is allowed only behind a future feature; `ndarray`/`linfa` not in default), advisories.
- `renovate.json` (or `dependabot.yml`) updates Cargo.lock weekly.
- CI job `cargo deny check`.

## Data flow

**Agent-driven lint in CI (#55, #46)**

1. `cxas actions init --app-dir ./pilot` writes the workflow.
2. On PR, the workflow runs `cxas lint --app-dir ./pilot --format json`.
3. The agent / CI parses `ok` and `data.diagnostics`.

**Trace debug (#350)**

1. `cxas trace --app-name ... --raw --format json`.
2. Consumer reads NDJSON; each line has `raw`.

**Versioned pull (#252)**

1. Resolve app name + `Location` from args or workspace.
2. `export_app_version` stream → zip extract into `--target-dir`.

**Deploy (#386)**

1. Resolve workspace profile.
2. Push bytes → create version → update deployment (channel settings optional).

## Error handling

| Condition | `error.code` | Exit |
|---|---|---|
| Clap usage / missing required flag | `USAGE` | 2 |
| CES `NOT_FOUND` | `CES_NOT_FOUND` | 1 |
| Lint engine I/O | `LINT_IO` | 1 |
| Lint diagnostics with severity Error | `LINT_ERRORS` | 1 |
| Eval overall FAIL | `EVAL_FAIL` | 1 |
| Drift detected | `DRIFT` | 1 |
| TTY required | `TTY_REQUIRED` | 2 |
| `--auto-create-wif` | `WIF_MANUAL` | 2 |
| Location missing after workspace resolve | `LOCATION_REQUIRED` | 2 |
| Feature not in this binary (`llm-lint` without `llm`) | `FEATURE_DISABLED` | 2 |

JSON is written to stdout on both success and failure so agents never have to scrape stderr. Human logs (progress) go to stderr.

The process never prompts when `--no-input` is true (the default).

## Testing

CLI tests use `assert_cmd` + `predicates` and inject a mock CES via an env var `CXAS_TRANSPORT=mock` plus an in-process mock registered when the `mock-transport` feature (test-only) is on. Alternatively, library functions `cxas_cli::run(argv, Box<dyn CesTransport>)` are unit-tested and the binary is a thin wrapper; **both** the library entry point and the binary `--help` are tested.

1. **#55** — `cxas --format json lint --app-dir <fixture>` stdout parses as JSON with `ok` bool; no interactive wait (test timeout 2s).
2. **#46 / #54** — `actions init` with an `environment.json` containing `{"dev":{},"prod":{}}` writes a workflow whose YAML contains a matrix with `dev` and `prod`. `init-github-action` produces the same files.
3. **#350** — `trace --raw` on a mock 2-turn session prints two JSON lines, each with a `raw` object.
4. **#386** — `deploy` on a fixture dir calls mock `import_app` + `create_version` + `create_deployment`.
5. **#252** — `pull --version-id v3` records `v3` on the mock export.
6. **#206** — `evals report --format json` includes `turns`.
7. **#131 / #270** — `state` prints hash + location from a fixture workspace file; `diff` exits 1 when a tool file differs.
8. **#99** — `deny.toml` exists; a unit test reads it and asserts `licenses` and `advisories` tables are present. (Running `cargo deny` is a CI step, not required on developer machines without the binary.)
9. **#298 through CLI** — mock export of 5 MiB succeeds via `pull`.
10. **Parity** — a test loads `cxas_parity::load_bundled()` and asserts every non-extension CLI argv is a clap command (`cxas_cli::build_parser().find_subcommand` walk).
11. **mdBook** — `book/src/SUMMARY.md` contains the four section titles Docs, Examples, Agent Skills, Core SDK.
12. **No implicit global** — `cxas pull` without `--location` and without a workspace file exits 2 with `LOCATION_REQUIRED`.

No live CES, no live GitHub API, no `cargo-dist` publish in unit tests.

## Out of scope

- Implementing the Gauntlet Loop orchestrator.
- Auto-creating GCP WIF resources.
- Porting every MkDocs page verbatim (structure is required; prose can reference these specs).
- Executing this plan's later packaging job in the Superpowers spec-writing goal.
