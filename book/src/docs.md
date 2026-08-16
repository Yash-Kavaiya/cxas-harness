# Docs

Published website: <https://yash-kavaiya.github.io/cxas-harness/>

That site is the full user and contributor documentation for `cxas-harness`. This mdBook is the Phase 5 sidebar (Docs / Examples / Agent Skills / Core SDK). Prefer the website for command flags, crate APIs, and issue coverage.

## In this repository

| Topic | Where |
|---|---|
| Getting started | website `getting-started.html`, root `README.md` |
| CLI | website `cli.html`, `crates/cxas-cli/src/args.rs` |
| Architecture | website `architecture.html` |
| Superpowers specs | `docs/superpowers/specs/` |
| Superpowers plans | `docs/superpowers/plans/` |
| Coverage map | `docs/superpowers/coverage-map.md` |
| Issue provenance | `GoogleCloudPlatform/cxas-scrapi` at SHA `4f7b43ca6adda0acad95a7e3654eee4e2ed1438c` |

## CLI contract

- Default `--format json`
- `--no-input` on
- Exit 0 success, 1 runtime/lint/eval/drift, 2 usage/TTY/feature
- Envelope: `{ "ok", "command", "data?", "error?" }`

## Location

`cxas_core::Location` has no `Default`. Empty is an error. `__default_global__` is rejected. `"global"` is allowed only when the caller passes it.

Packaging and CI design: `docs/superpowers/specs/2026-08-15-packaging-cli-ci-design.md`.
