# Discovery benchmark and Gauntlet Loop — design

Date: 2026-08-15
Status: approved for planning
Supersedes: the "no Gauntlet plan is required" ruling in `docs/superpowers/coverage-map.md`

## Purpose

`cxas-harness` currently passes 78 tests against a benchmark it wrote for itself. This
design replaces that self-graded contract with the canonical Google CES API discovery
documents, then builds the Gauntlet Loop on top of the new benchmark so that builder
agents are scored by blind critics against evidence rather than against their own claims.

Three phases, in order:

1. **Benchmark** — vendor the CES discovery documents, generate the API model from them,
   and replace the self-referential parity test with one that can fail.
2. **Gauntlet** — build the builder/blind-critic loop as repo tooling under `gauntlet/`.
3. **Transport** — let the loop drive a real REST/JSON client against CES.

## Motivating defect

The review that produced this design found that `crates/cxas-proto/src/evaluation_run_state.rs`
declares:

```
Unspecified, Pending, Running, Succeeded, Failed, Cancelled
```

The CES `v1beta` discovery document (revision `20260730`) declares
`EvaluationRun.state` as:

```
EVALUATION_RUN_STATE_UNSPECIFIED, QUEUED, RUNNING, COMPLETED, ERROR, CANCELLED
```

Three of six variants are invented, and `as_str_name()` emits `"PENDING"`, `"SUCCEEDED"`,
and `"FAILED"` — strings absent from the CES API. The test that closes issue #284, the
enum-drift bug, has itself drifted. The `Unknown(i32)` fallback is correct and is retained;
only the variant set was guessed.

This defect is the design's justification. It is invisible to 78 passing tests and obvious
to a critic holding the discovery document.

## Global constraints

- **The discovery documents are the sole authority on what the CES API is.** Where the
  Python `cxas-scrapi` surface disagrees, CES wins and the divergence is recorded as a
  deliberate fix.
- **The Python surface remains authoritative for CLI shape only** — command names, argv
  structure, and output ergonomics.
- **No network at build time.** Generated code and vendored references are checked in.
- **The Gauntlet Loop never ships in the `cxas` binary.** It is repo tooling.
- **`Location` stays mandatory.** Every discovery path template embeds `locations/*`,
  which makes issue #401 structurally unrepeatable.

## Architecture

```
reference/
  ces/
    v1.discovery.json          canonicalized, checked in
    v1beta.discovery.json      canonicalized, checked in
    PINNED.toml                revision, source URL, sha256 per file
  python/
    surface.yaml               today's parity YAML, demoted to CLI/UX reference
crates/
  cxas-discovery/              new: pure parser, discovery JSON -> typed model
xtask/                         new: codegen, discovery -> enums + REST client
gauntlet/                      new: orchestrator, roles, evidence collector
```

## Data flow

```
reference/ces/*.json
        |
        v
  cxas-discovery  (parse -> Discovery { methods, schemas, enums })
        |
        +--> xtask codegen ------> generated enums + typed REST client (checked in)
        |
        +--> cxas-parity tests --> enum parity, method resolution, coverage report
        |
        +--> gauntlet evidence --> coverage diff handed to blind critics
```

`cxas-discovery` is the single definition of "what the API is". The generator, the parity
tests, and the Gauntlet evidence collector all read through it, so the three cannot drift
from one another.

### Canonicalization

The discovery endpoint does not guarantee stable key ordering between fetches. Vendored
files are canonicalized on write — keys sorted, two-space indent, trailing newline — so
that the recorded sha256 is reproducible and drift diffs are readable. `PINNED.toml`
records the source URL, the API `revision` field, and the canonicalized sha256.

## Components

### `reference/` (data)

Checked-in, not generated at build time. Refreshed only by an explicit
`xtask refresh-reference` invocation, which rewrites the canonicalized JSON and
`PINNED.toml` together. A CI job re-fetches, canonicalizes, and fails if the result
differs from what is checked in — turning upstream API drift into a reviewable pull
request rather than a silent behavioural change.

### `crates/cxas-discovery`

Pure parser over the vendored JSON. No network, no codegen, no CES semantics.

```rust
pub struct Discovery { pub version: String, pub revision: String, /* ... */ }
pub struct Method { pub id: String, pub http_method: String, pub path: String, /* ... */ }
pub struct Schema { pub id: String, pub properties: Vec<Property> }
pub struct EnumField { pub schema: String, pub property: String, pub values: Vec<String> }

impl Discovery {
    pub fn load(path: &Path) -> Result<Self, DiscoveryError>;
    pub fn method(&self, id: &str) -> Option<&Method>;
    pub fn enum_field(&self, schema: &str, property: &str) -> Option<&EnumField>;
    pub fn methods(&self) -> impl Iterator<Item = &Method>;
}
```

What it does: turns two JSON files into a queryable model. How you use it: `load`, then
`method`/`enum_field` lookups. What it depends on: `serde_json` and the vendored files,
nothing else. It is testable with a small fixture discovery document and has no knowledge
of `cxas-core`, the CLI, or the Gauntlet Loop.

### `xtask` (code generation)

Generates two artifacts from `cxas-discovery`, both checked in:

- **Enum tables** for `cxas-proto` — exact wire spellings, `Unknown(i32)` fallback
  preserved, `as_str_name()` returning the real CES strings.
- **Typed REST client** for `cxas-core` — request/response structs and one method per
  discovery method, path templates rendered from typed parameters.

`xtask verify-codegen` regenerates into a temporary directory and diffs against the
checked-in output; CI fails on any difference. This keeps generated code reviewable
while making it impossible to hand-edit without detection.

Generation is scoped, not all-at-once. Phase 3 gates on the `apps`, `agents`, `tools`,
and v1beta evaluation resources before extending to the full 170-method surface.

### `crates/cxas-parity` (rebuilt contract)

The existing `manifest_contract.rs` asserts that a checked-in YAML contains strings that
same YAML declares. It is replaced by three tests with real failure modes:

| Test | Asserts | Initial state |
|---|---|---|
| `enum_variants_match_discovery` | every generated enum's variant set and wire spelling equals discovery | **fails** on `EvaluationRunState` |
| `declared_methods_resolve_in_discovery` | every method `cxas-core` claims maps to a real discovery method id | passes trivially, gains teeth in phase 3 |
| `coverage_report` | emits implemented-method counts per version (66 in v1, 104 in v1beta, 170 combined) as machine-readable output | reports, does not gate |

`coverage_report` deliberately does not gate. A pass/fail coverage threshold can be
satisfied by deleting the metric; a reported number cannot. The Gauntlet critics consume
this number as evidence.

The Python manifest tests are retained, renamed to `python_surface_*`, and rescoped to
CLI argv and command naming only.

### `gauntlet/` (the loop)

```
gauntlet/
  config.toml          agent command, pieces, caps, budget
  orchestrator.py      plan, fan out, merge. Never implements, never critiques.
  evidence.py          deterministic evidence bundle builder
  agents/
    builder.md         builder role prompt
    critic.md          blind critic role prompt
  runs/<id>/
    scorecard.json     live status: piece, round, score, verdict
    evidence/          one bundle per round, retained for audit
```

**Provider-agnostic execution.** `config.toml` declares `agent_cmd` (for example
`claude -p`, `gemini -p`, or `codex exec`). The orchestrator writes a prompt to the
subprocess's stdin and reads its stdout. No provider SDK is imported, and swapping
providers is a one-line config change.

**Role separation.** Three roles, enforced structurally rather than by instruction:

- *Orchestrator* — decomposes, dispatches, merges. It is the only role with merge
  authority and it never writes crate code.
- *Builder* — one per piece, working in its own git worktree, committing before
  requesting critique so failures stay isolated.
- *Critic* — receives an evidence bundle and nothing else.

**The evidence bundle** is the load-bearing artifact. It is produced by `evidence.py`,
which is deterministic code, not an agent:

| Included | Excluded |
|---|---|
| `cargo test` output | crate source code |
| `cargo clippy` output | the builder's rationale or commit messages |
| discovery coverage diff (implemented vs 170, enum mismatches) | any self-assessment by the builder |
| issue-repro result for the piece's assigned issues | |
| binary size and build time | |

Because the critic never sees the source or the rationale, it cannot be argued into
accepting an explanation in place of a result. It returns a score, **the single biggest
remaining gap**, and a verdict. It may not implement and may not narrow scope. Its stated
gap becomes the builder's next prompt.

**Pieces** map onto the existing crate boundaries, so the loop grafts onto the current
workspace rather than reorganising it.

**Stop conditions**, stated explicitly to prevent runaway iteration:

- per-piece round cap (`max_rounds`)
- global budget (`budget_usd`, `0` meaning unlimited)
- a release-candidate gate requiring all 25 catalogued issues to have a repro test,
  100% enum parity against discovery (all 132 enum-bearing fields across both versions),
  and method coverage at or above `rc_coverage_min` in `config.toml`. Phase 3 sets that
  value to the gated subset — `apps`, `agents`, `tools`, and the v1beta evaluation
  resources — not to all 170 methods.

On reaching the RC gate the orchestrator stops and hands back for human review.

### Transport (phase 3)

REST/JSON over `reqwest`, generated from the same discovery documents. Authentication via
Application Default Credentials or an explicit `--oauth-token`. `CesTransport` grows from
its current single method to the generated surface. `NoopTransport` and
`RecordingTransport` are retained as test doubles.

v1beta evaluation resources land first, because issues #284, #263, #355, #345, and #136
all live there. The v1 surface has no evaluation resources at all; the current code models
neither.

## Error handling

- **Missing or corrupt reference file** — `cxas-discovery` returns `DiscoveryError::Io`
  or `DiscoveryError::Parse`. Never a panic, never a silent empty model, because an empty
  model would make every coverage and parity test pass vacuously.
- **Unknown enum value on the wire** — maps to `Unknown(i32)`, preserving the #284 fix.
  Callers match exhaustively; no name lookup on a raw integer.
- **Upstream drift** — the CI refresh job fails with a diff. Never auto-merged.
- **Agent subprocess failure** — the orchestrator records a failed round with the captured
  stderr in the scorecard and either retries within the round cap or marks the piece
  blocked. A crashed agent never counts as a passing critique.
- **Critic returning malformed output** — treated as a failed round, not as approval.
  Silence is not consent.

## Testing

- `cxas-discovery` — fixture-based unit tests over a small hand-written discovery
  document, covering parse, lookup, and both error paths.
- `xtask` — golden-file tests: generation from the fixture produces expected output;
  `verify-codegen` detects a hand-edit.
- `cxas-parity` — the three contract tests above. `enum_variants_match_discovery` is
  written first and must fail before the enum fix lands.
- `gauntlet/evidence.py` — unit tests asserting the bundle contains the required keys and,
  critically, that it **excludes** source code and rationale. The exclusion test is what
  keeps the critic blind as the code evolves.
- `gauntlet/orchestrator.py` — tested against a stub `agent_cmd` (a script emitting canned
  verdicts), so the loop is testable without invoking a real model.

## Consequences accepted

- **Fixing `EvaluationRunState` breaks currently-green tests** that assert `"SUCCEEDED"`.
  The test count dips before it climbs. This is the correct outcome and is not to be
  worked around by keeping the invented variants as aliases.
- **170 methods is a large generated surface.** Phase 3 is gated to apps, agents, tools,
  and v1beta evaluations before extending further.
- **`v1beta` is unstable by definition.** The pinned revision plus the CI drift check is
  the mitigation; there is no way to make an unstable upstream stable.
- **The coverage number will look bad at first** — honest surface coverage today is under
  1%. Reporting it accurately is the point.

## Issue-driven quality bar

The existing 25-issue closure bar from `docs/superpowers/coverage-map.md` is retained and
tightened: a closing test now counts only if it exercises behaviour verified against the
discovery document, not against a test double asserting the code's own assumptions. The
#284 closer is the first to be re-tested under the tightened bar, and it currently fails it.
