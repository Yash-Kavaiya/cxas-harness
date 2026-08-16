# Gauntlet Loop

Builder/blind-critic loop for `cxas-harness`. **Repo tooling — never shipped in
the `cxas` binary.** Nothing here is a Cargo workspace member.

## Run

```sh
python gauntlet/orchestrator.py             # every piece in config.toml
python gauntlet/orchestrator.py cxas-proto  # one piece
```

## Configure

Edit `gauntlet/config.toml`. `agent_cmd` is any CLI that reads a prompt on stdin
and writes a response on stdout — `claude -p`, `gemini -p`, `codex exec`.
Swapping providers is a one-line change; no provider SDK is imported.

## Design

Three roles, separated structurally rather than by instruction:

| Role | Sees | May |
|---|---|---|
| Orchestrator | scorecards | plan, fan out, merge — never implement or critique |
| Builder | its own crate, the critic's last gap | edit that crate only |
| Critic | the evidence bundle, nothing else | score and name one gap — never implement |

The evidence bundle is built by `gauntlet/evidence.py` — deterministic code,
not an agent — *after* the builder finishes, so the builder cannot shape what
the critic sees.

| Critic receives | Critic never receives |
|---|---|
| `cargo test` output | crate source code |
| `cargo clippy` output | the diff |
| CES discovery coverage and revisions | commit messages |
| assigned-issue reproductions | the builder's rationale |
| binary size | any self-assessment |

That blindness is asserted, not merely documented:
`test_bundle_excludes_source_code` and `test_rendered_bundle_contains_no_rust_source`
fail if source ever reaches a critic. If it did, the loop would degenerate into
self-grading — the exact failure this design exists to prevent.

## Why the benchmark matters

A critic is only as sharp as its reference. This loop's reference is Google's
own CES discovery documents, vendored under `reference/ces/` at a pinned
revision: 66 methods in v1, 104 in v1beta.

The first thing that benchmark caught was `EvaluationRunState` declaring
`PENDING`/`SUCCEEDED`/`FAILED` where CES declares `QUEUED`/`COMPLETED`/`ERROR`.
The test that supposedly closed the enum-drift bug (#284) had itself drifted,
and 78 passing tests could not see it.

## Stop conditions

The loop has no natural finish line, so the stop conditions are explicit:

| Setting | Effect |
|---|---|
| `max_rounds` | per-piece iteration cap |
| `max_agent_calls` | hard cap on total agent invocations for the whole run, shared across pieces; two per round, `0` = unlimited |
| `rc_coverage_min` | a clean sweep below this many declared CES methods exits non-zero as "not a release candidate" |

Reaching a cap is a `FAIL`, never a pass. A round that cannot afford both its
calls is not started at all: spending the builder call and then stopping would
leave the crate edited and unreviewed, which is the worst of both.

There is deliberately no `budget_usd`. `agent_cmd` is any CLI that reads stdin
and writes stdout, so no provider ever reports a cost back and a dollar figure
here could only ever have been decorative — it was, until 2026-08-16, when it
was replaced by a cap the loop can actually enforce. `test_stop_conditions.py`
fails if `budget_usd` ever returns.

## Failure handling

A round fails — never silently passes — when the critic returns unparseable
output, returns nothing, or when the configured agent binary is missing or
times out. Silence is not consent.

## Test the loop itself

```sh
python -m pytest gauntlet/tests/ -v
```

Tests run against `gauntlet/tests/stub_agent.py`, so the loop is verifiable
without invoking a real model or spending tokens.
