# cxas-harness

[![Documentation](https://img.shields.io/badge/docs-yash--kavaiya.github.io-d28a4c?style=flat-square)](https://yash-kavaiya.github.io/cxas-harness/)
[![GitHub](https://img.shields.io/badge/github-Yash--Kavaiya%2Fcxas--harness-171c24?style=flat-square)](https://github.com/Yash-Kavaiya/cxas-harness)

Rust rewrite of Google Cloud's [`cxas-scrapi`](https://github.com/GoogleCloudPlatform/cxas-scrapi) — a CLI and library harness for CX Agent Studio (CES).

**Full documentation: <https://yash-kavaiya.github.io/cxas-harness/>**

```sh
cargo test --workspace
cargo run -p cxas-cli -- --help
```

## What it is

A machine-first `cxas` binary — JSON by default, `--no-input` on, stable exit codes — over ten workspace crates.

All **170 methods** CES declares are addressable, from a table generated out of Google's own discovery documents; **37** are additionally modelled with this workspace's own types and CLI verbs. The two numbers are reported separately, because generating 170 path templates is cheap and deciding what a `Deployment` is, and what happens when promoting one fails, is not.

Location is never defaulted to `"global"`. Every CES path template embeds `projects/*/locations/*`, so a resource name cannot be built without one.

| | Where |
|---|---|
| Every CLI command | [cli.html](https://yash-kavaiya.github.io/cxas-harness/cli.html) |
| Crate-by-crate SDK | [crates.html](https://yash-kavaiya.github.io/cxas-harness/crates.html) |
| Architecture and data flow | [architecture.html](https://yash-kavaiya.github.io/cxas-harness/architecture.html) |
| Benchmark and Gauntlet Loop | [benchmark.html](https://yash-kavaiya.github.io/cxas-harness/benchmark.html) |
| **What this does *not* do** | [limits.html](https://yash-kavaiya.github.io/cxas-harness/limits.html) |

## Requirements

- Rust 1.80+ (`rustc` / `cargo` on `PATH`, or `%USERPROFILE%\.cargo\bin` on Windows)
- Git
- Optional: `protoc` (without it, `cxas-proto` uses the hand-written `EvaluationRunState` wrapper)

## Build and test

```sh
cargo test --workspace          # 221 tests
cargo clippy --workspace --all-targets
cargo build -p cxas-cli
```

```powershell
# Windows PowerShell, if cargo is not on PATH
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
```

Python-side tooling — the reference refresher, the method-table generator, and the Gauntlet loop:

```sh
python -m pytest tests gauntlet/tests -q   # 45 tests
```

## Run the CLI

```sh
cxas init --app-dir ./my-app
cxas lint --app-dir ./my-app
cxas state --app-dir ./my-app --location us-central1 --project-id my-project
```

JSON is the default; `--format human` for people. Local apps persist in `.cxas/catalog.json` (override with `CXAS_CATALOG`). Exit codes are 0 success, 1 runtime failure, 2 usage — the full contract is on [cli.html](https://yash-kavaiya.github.io/cxas-harness/cli.html).

## Talking to CES

`cxas-core` carries every method CES declares as a generated table, and `cxas-parity` asserts the table and the vendored discovery documents agree **in both directions** — a method CES added that the table lacks fails just as loudly as a path the table invented.

```text
CES-COVERAGE addressable v1=66/66 v1beta=104/104 total=170/170 modelled=37/170
```

Evaluations are registered only against `v1beta`, because the `v1` surface exposes no evaluation resources at all. A test enforces that, since declaring one on `v1` would build a URL that can only ever 404.

```sh
cxas api list --filter evaluationRuns          # what exists, offline
cxas api describe ces.projects.locations.apps.get

cxas api call ces.projects.locations.apps.list \
  --param parent=projects/my-project/locations/us-central1 \
  --query pageSize=25

cxas api stream ces.projects.locations.apps.sessions.streamRunSession \
  --param session=projects/my-project/locations/us-central1/apps/demo/sessions/s1 \
  --body '{"query":"hello"}'
```

A missing path parameter is named before anything is sent, so the reply is the parameter name rather than a 404 from CES.

**Credentials** resolve the way Google's own tools resolve them: `--oauth-token`, then `CXAS_ACCESS_TOKEN`, then `GOOGLE_APPLICATION_CREDENTIALS`, then the well-known ADC file, then the metadata server, then `gcloud auth print-access-token`. Tokens are cached and refreshed a minute before expiry.

Service-account key files and workload-identity federation are **not** implemented — both need signing this workspace does not do. An unusable credential at a higher precedence is an error rather than a reason to fall through: silently authenticating as the developer when the operator configured a robot would send the request as the wrong principal and make the resulting 403 point at the wrong problem.

**Streaming.** `streamRunSession` is delivered message by message as it arrives. A stream that ends mid-message is an error rather than a short result — a dropped connection and a finished conversation are otherwise indistinguishable — and whatever did arrive whole is still reported.

Request construction is pure and always compiled; only the send step needs the `rest` feature:

```rust
use cxas_core::{method_spec, ApiVersion, AppRef, CesHttpClient, Location};

let app = AppRef::new("my-project", Location::new("us-central1")?, "my-app")?;
let client = CesHttpClient::discover(None)?;   // ADC, cached and refreshed
let spec = method_spec("ces.projects.locations.apps.get", ApiVersion::V1).unwrap();
let json = client.call(spec, &params, &query, None).await?;
```

## Benchmark and the Gauntlet Loop

The benchmark is Google's own CES discovery documents, vendored under [`reference/ces/`](reference/ces/) at a pinned revision and verified by sha256.

That contract found a real defect on its first run. `EvaluationRunState` declared `PENDING`/`SUCCEEDED`/`FAILED` where CES declares `QUEUED`/`COMPLETED`/`ERROR` — the test closing the enum-drift bug ([#284](https://github.com/GoogleCloudPlatform/cxas-scrapi/issues/284)) had itself drifted, invisibly to 78 passing tests. The previous parity contract could not have caught it: it asserted that a checked-in YAML contained strings that same YAML declared.

```sh
python tools/refresh_reference.py --check     # fail if upstream is newer
python tools/generate_methods.py --check      # fail if the table is stale
```

[`gauntlet/`](gauntlet/) builds on that benchmark: builder agents work per crate, each paired with a blind critic that sees only test output, clippy results, discovery coverage, and issue reproductions — never the source or the builder's reasoning. That blindness is enforced by a test, not by instruction. See [`gauntlet/README.md`](gauntlet/README.md).

## Honesty

Every test here runs against loopback stubs and vendored documents. That keeps the suite deterministic and offline, and it also means **no request in this codebase has ever been answered by CES itself**. The URLs, verbs, and enum spellings match Google's machine-readable description of the API — a strong claim, and a different one from "this works against production".

[limits.html](https://yash-kavaiya.github.io/cxas-harness/limits.html) is the full list of what is missing: 133 methods with no typed body, no retries, no long-running-operation polling, catalog commands still on `.cxas/catalog.json`, no release packaging.

## Design docs

- Specs: [`docs/superpowers/specs/`](docs/superpowers/specs/)
- Plans: [`docs/superpowers/plans/`](docs/superpowers/plans/)
- Coverage map: [`docs/superpowers/coverage-map.md`](docs/superpowers/coverage-map.md) — every phase and all 25 cataloged `cxas-scrapi` issues, mapped to a spec section and a plan task

## License

Apache-2.0. This project is an independent rewrite; it is not an official Google product.
