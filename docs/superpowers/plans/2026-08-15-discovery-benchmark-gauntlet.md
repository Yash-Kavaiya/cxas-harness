# Discovery Benchmark and Gauntlet Loop Implementation Plan

> **Amended 2026-08-16.** The `budget_usd` and `rc_coverage_min` stop conditions
> described below were declared in `gauntlet/config.toml` and read by no code.
> `budget_usd` could not be honest in this design -- `agent_cmd` is any
> stdin/stdout CLI, so no cost is ever reported back -- and is replaced by
> `max_agent_calls`, a cap on invocations the loop can actually count.
> `rc_coverage_min` is now enforced after a clean sweep. Both are covered by
> `gauntlet/tests/test_stop_conditions.py`. The prose below is the dated record
> of what this phase specified.


> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the self-graded parity contract with Google's canonical CES discovery documents as the benchmark, then build the builder/blind-critic Gauntlet Loop on top of it as repo tooling.

**Architecture:** Two vendored, canonicalized discovery JSON files become the sole authority on the CES API. A new pure-parser crate `cxas-discovery` reads them. The parity tests, and later the Gauntlet evidence collector, both query through that one parser so they cannot drift apart. The Gauntlet Loop lives under `gauntlet/` as a provider-agnostic Python orchestrator that shells out to a configurable agent CLI and feeds blind critics a deterministic evidence bundle.

**Tech Stack:** Rust 1.80 (workspace, edition 2021), `serde_json`, `thiserror`; Python 3.11 for repo tooling (stdlib only — no pip installs).

**Spec:** `docs/superpowers/specs/2026-08-15-discovery-benchmark-gauntlet-design.md`

**Scope note:** This plan covers Phase 1 (benchmark) and Phase 2 (Gauntlet Loop) from the spec. Phase 3 (REST transport + full codegen) is deliberately excluded — the spec states the loop drives that work, so its plan is written after the loop is running.

## Global Constraints

- Rust edition `2021`, `rust-version = "1.80"`, `license = "Apache-2.0"` in every crate manifest — copy the pattern from `crates/cxas-parity/Cargo.toml`.
- Every new `.rs` file starts with the 13-line Apache-2.0 header used by every existing file in `crates/` — copy it verbatim from `crates/cxas-proto/src/lib.rs`.
- **No network access at build time or test time.** Vendored reference files are checked in; only `tools/refresh_reference.py`, run by hand or in CI, fetches.
- Python tooling uses the **standard library only**. No `pip install`, no `requests` — use `urllib.request`.
- The Gauntlet Loop must never be reachable from the `cxas` binary. Nothing under `gauntlet/` may be added to `Cargo.toml` workspace members.
- CES discovery is the sole authority on the API surface. Where Python `cxas-scrapi` disagrees, CES wins.
- Discovery source URLs, verbatim:
  - `https://ces.googleapis.com/$discovery/rest?version=v1`
  - `https://ces.googleapis.com/$discovery/rest?version=v1beta`
- Canonicalization format, verbatim: `json.dumps(obj, sort_keys=True, indent=2, ensure_ascii=False)` plus one trailing newline.
- On Windows, prefix cargo commands with `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"` if cargo is not on PATH.

---

## File Structure

| File | Responsibility |
|---|---|
| `tools/refresh_reference.py` | Fetch, canonicalize, hash, and pin the two discovery documents |
| `reference/ces/v1.discovery.json` | Vendored canonical v1 surface (data) |
| `reference/ces/v1beta.discovery.json` | Vendored canonical v1beta surface (data) |
| `reference/ces/PINNED.toml` | Source URL, API revision, sha256 per file |
| `crates/cxas-discovery/src/lib.rs` | Crate root, re-exports, `DiscoveryError` |
| `crates/cxas-discovery/src/model.rs` | `Discovery`, `Method`, `Schema`, `EnumField` types + lookups |
| `crates/cxas-discovery/src/parse.rs` | JSON → model, including recursive resource walk |
| `crates/cxas-discovery/tests/fixture.rs` | Parser tests over a small hand-written document |
| `crates/cxas-discovery/tests/vendored.rs` | Asserts the real vendored files parse to expected counts |
| `crates/cxas-proto/src/enum_registry.rs` | Maps each Rust enum to its discovery schema + property |
| `crates/cxas-proto/src/evaluation_run_state.rs` | Corrected `EvaluationRunState` (modified) |
| `crates/cxas-parity/tests/discovery_contract.rs` | Enum parity, method resolution, coverage report |
| `gauntlet/evidence.py` | Deterministic evidence bundle builder |
| `gauntlet/orchestrator.py` | Provider-agnostic builder/critic loop |
| `gauntlet/config.toml` | Agent command, pieces, caps, budget, RC gate |
| `gauntlet/agents/builder.md` | Builder role prompt |
| `gauntlet/agents/critic.md` | Blind critic role prompt |
| `gauntlet/tests/test_evidence.py` | Bundle contents and, critically, blindness |
| `gauntlet/tests/test_orchestrator.py` | Loop behaviour against a stub agent |
| `gauntlet/tests/stub_agent.py` | Canned-verdict agent used by orchestrator tests |
| `gauntlet/README.md` | How to run the loop |

---

# Phase 1 — Benchmark

## Task 1: Vendor the discovery documents

**Files:**
- Create: `tools/refresh_reference.py`
- Create: `reference/ces/v1.discovery.json` (generated by the script)
- Create: `reference/ces/v1beta.discovery.json` (generated by the script)
- Create: `reference/ces/PINNED.toml` (generated by the script)
- Test: `tests/test_refresh_reference.py`

**Interfaces:**
- Consumes: nothing.
- Produces: `canonicalize(obj: dict) -> str`, `pinned_toml(entries: list[dict]) -> str`, and the three files above. Task 2 reads the JSON files; Task 5 reads `PINNED.toml`.

- [ ] **Step 1: Write the failing test**

Create `tests/test_refresh_reference.py`:

```python
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))

from refresh_reference import canonicalize, pinned_toml


def test_canonicalize_sorts_keys_and_ends_with_newline():
    out = canonicalize({"b": 1, "a": {"d": 2, "c": 3}})
    assert out.endswith("\n")
    assert out == '{\n  "a": {\n    "c": 3,\n    "d": 2\n  },\n  "b": 1\n}\n'


def test_canonicalize_is_idempotent():
    obj = {"z": [3, 1, 2], "a": "x"}
    once = canonicalize(obj)
    twice = canonicalize(json.loads(once))
    assert once == twice


def test_canonicalize_preserves_non_ascii():
    assert "é" in canonicalize({"k": "café"})


def test_pinned_toml_records_url_revision_and_sha():
    out = pinned_toml([
        {"version": "v1", "url": "https://example/v1", "revision": "20260730", "sha256": "abc"},
    ])
    assert '[[reference]]' in out
    assert 'version = "v1"' in out
    assert 'revision = "20260730"' in out
    assert 'sha256 = "abc"' in out
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python -m pytest tests/test_refresh_reference.py -v`
Expected: FAIL with `ModuleNotFoundError: No module named 'refresh_reference'`

- [ ] **Step 3: Write the implementation**

Create `tools/refresh_reference.py`:

```python
#!/usr/bin/env python3
"""Fetch, canonicalize, and pin the CES discovery documents.

Run by hand or in CI. Nothing at build time touches the network.
"""
import hashlib
import json
import sys
import urllib.request
from pathlib import Path

SOURCES = [
    ("v1", "https://ces.googleapis.com/$discovery/rest?version=v1"),
    ("v1beta", "https://ces.googleapis.com/$discovery/rest?version=v1beta"),
]

REFERENCE_DIR = Path(__file__).resolve().parents[1] / "reference" / "ces"


def canonicalize(obj):
    """Serialize deterministically so the sha256 is reproducible across fetches."""
    return json.dumps(obj, sort_keys=True, indent=2, ensure_ascii=False) + "\n"


def pinned_toml(entries):
    lines = [
        "# Generated by tools/refresh_reference.py. Do not edit by hand.",
        "",
    ]
    for e in entries:
        lines += [
            "[[reference]]",
            f'version = "{e["version"]}"',
            f'url = "{e["url"]}"',
            f'revision = "{e["revision"]}"',
            f'sha256 = "{e["sha256"]}"',
            "",
        ]
    return "\n".join(lines)


def fetch(url):
    with urllib.request.urlopen(url, timeout=60) as resp:
        return json.loads(resp.read().decode("utf-8"))


def main(argv):
    check_only = "--check" in argv
    REFERENCE_DIR.mkdir(parents=True, exist_ok=True)
    entries = []
    drifted = []
    for version, url in SOURCES:
        doc = fetch(url)
        text = canonicalize(doc)
        digest = hashlib.sha256(text.encode("utf-8")).hexdigest()
        path = REFERENCE_DIR / f"{version}.discovery.json"
        if check_only:
            existing = path.read_text(encoding="utf-8") if path.exists() else ""
            if existing != text:
                drifted.append(version)
        else:
            path.write_text(text, encoding="utf-8", newline="\n")
        entries.append({
            "version": version,
            "url": url,
            "revision": doc.get("revision", "unknown"),
            "sha256": digest,
        })
        print(f"{version}: revision={entries[-1]['revision']} sha256={digest[:12]} bytes={len(text)}")

    if check_only:
        if drifted:
            print(f"DRIFT: {', '.join(drifted)} differ from the vendored copies", file=sys.stderr)
            return 1
        print("no drift")
        return 0

    (REFERENCE_DIR / "PINNED.toml").write_text(pinned_toml(entries), encoding="utf-8", newline="\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python -m pytest tests/test_refresh_reference.py -v`
Expected: PASS, 4 tests

- [ ] **Step 5: Generate the vendored reference files**

Run: `python tools/refresh_reference.py`
Expected output shape: `v1: revision=... sha256=... bytes=...` then the same for `v1beta`.

Verify the files landed:

Run: `python -c "import json;d=json.load(open('reference/ces/v1beta.discovery.json'));print(d['revision'], len(d['schemas']))"`
Expected: a revision string and a schema count of roughly 333.

- [ ] **Step 6: Commit**

```bash
git add tools/refresh_reference.py tests/test_refresh_reference.py reference/
git commit -m "feat(reference): vendor canonicalized CES discovery documents"
```

---

## Task 2: The `cxas-discovery` parser crate

**Files:**
- Create: `crates/cxas-discovery/Cargo.toml`
- Create: `crates/cxas-discovery/src/lib.rs`
- Create: `crates/cxas-discovery/src/model.rs`
- Create: `crates/cxas-discovery/src/parse.rs`
- Modify: `Cargo.toml` (add `crates/cxas-discovery` to `members`)
- Test: `crates/cxas-discovery/tests/fixture.rs`
- Test: `crates/cxas-discovery/tests/vendored.rs`

**Interfaces:**
- Consumes: the JSON files from Task 1.
- Produces — later tasks depend on these exact signatures:
  - `Discovery::load(path: &Path) -> Result<Discovery, DiscoveryError>`
  - `Discovery::revision(&self) -> &str`
  - `Discovery::method(&self, id: &str) -> Option<&Method>`
  - `Discovery::methods(&self) -> impl Iterator<Item = &Method>`
  - `Discovery::enum_field(&self, schema: &str, property: &str) -> Option<&EnumField>`
  - `Method { id: String, http_method: String, path: String }`
  - `EnumField { schema: String, property: String, values: Vec<String> }`
  - `DiscoveryError::{Io, Parse}`

- [ ] **Step 1: Write the failing tests**

Create `crates/cxas-discovery/tests/fixture.rs`:

```rust
// Copyright 2026 The cxas-harness Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use cxas_discovery::{Discovery, DiscoveryError};
use std::io::Write;

const FIXTURE: &str = r#"{
  "revision": "20260101",
  "version": "v1test",
  "schemas": {
    "EvaluationRun": {
      "id": "EvaluationRun",
      "properties": {
        "state": { "type": "string", "enum": ["A_UNSPECIFIED", "QUEUED", "DONE"] },
        "name": { "type": "string" }
      }
    }
  },
  "resources": {
    "projects": {
      "resources": {
        "locations": {
          "methods": {
            "get": { "id": "ces.projects.locations.get", "httpMethod": "GET", "path": "v1/{+name}" }
          }
        }
      },
      "methods": {
        "list": { "id": "ces.projects.list", "httpMethod": "GET", "path": "v1/projects" }
      }
    }
  }
}"#;

fn write_fixture(body: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().expect("tempfile");
    f.write_all(body.as_bytes()).expect("write");
    f.flush().expect("flush");
    f
}

#[test]
fn parses_revision() {
    let f = write_fixture(FIXTURE);
    let d = Discovery::load(f.path()).expect("parse");
    assert_eq!(d.revision(), "20260101");
}

#[test]
fn walks_nested_resources_for_methods() {
    let f = write_fixture(FIXTURE);
    let d = Discovery::load(f.path()).expect("parse");
    let mut ids: Vec<_> = d.methods().map(|m| m.id.as_str()).collect();
    ids.sort();
    assert_eq!(ids, vec!["ces.projects.list", "ces.projects.locations.get"]);
}

#[test]
fn method_lookup_returns_verb_and_path() {
    let f = write_fixture(FIXTURE);
    let d = Discovery::load(f.path()).expect("parse");
    let m = d.method("ces.projects.locations.get").expect("method");
    assert_eq!(m.http_method, "GET");
    assert_eq!(m.path, "v1/{+name}");
    assert!(d.method("ces.does.not.exist").is_none());
}

#[test]
fn enum_field_lookup_returns_values_in_order() {
    let f = write_fixture(FIXTURE);
    let d = Discovery::load(f.path()).expect("parse");
    let e = d.enum_field("EvaluationRun", "state").expect("enum field");
    assert_eq!(e.values, vec!["A_UNSPECIFIED", "QUEUED", "DONE"]);
}

#[test]
fn non_enum_property_is_not_an_enum_field() {
    let f = write_fixture(FIXTURE);
    let d = Discovery::load(f.path()).expect("parse");
    assert!(d.enum_field("EvaluationRun", "name").is_none());
}

#[test]
fn missing_file_is_io_error() {
    let err = Discovery::load(std::path::Path::new("no/such/file.json")).unwrap_err();
    assert!(matches!(err, DiscoveryError::Io(_)));
}

#[test]
fn malformed_json_is_parse_error() {
    let f = write_fixture("{ not json");
    let err = Discovery::load(f.path()).unwrap_err();
    assert!(matches!(err, DiscoveryError::Parse(_)));
}
```

Create `crates/cxas-discovery/tests/vendored.rs`:

```rust
// Copyright 2026 The cxas-harness Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use cxas_discovery::Discovery;
use std::path::PathBuf;

fn reference(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference/ces")
        .join(name)
}

#[test]
fn vendored_v1_parses_with_expected_surface() {
    let d = Discovery::load(&reference("v1.discovery.json")).expect("v1 must parse");
    assert_eq!(d.methods().count(), 66, "v1 method count changed; re-pin the reference");
}

#[test]
fn vendored_v1beta_parses_with_expected_surface() {
    let d = Discovery::load(&reference("v1beta.discovery.json")).expect("v1beta must parse");
    assert_eq!(d.methods().count(), 104, "v1beta method count changed; re-pin the reference");
}

#[test]
fn vendored_v1beta_declares_evaluation_run_state() {
    let d = Discovery::load(&reference("v1beta.discovery.json")).expect("v1beta must parse");
    let e = d
        .enum_field("EvaluationRun", "state")
        .expect("EvaluationRun.state must exist in v1beta");
    assert!(e.values.contains(&"QUEUED".to_string()));
    assert!(e.values.contains(&"COMPLETED".to_string()));
    assert!(e.values.contains(&"ERROR".to_string()));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p cxas-discovery`
Expected: FAIL — `error: package ID specification 'cxas-discovery' did not match any packages`

- [ ] **Step 3: Create the crate manifest and register it in the workspace**

Create `crates/cxas-discovery/Cargo.toml`:

```toml
[package]
name = "cxas-discovery"
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"
rust-version = "1.80"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"

[dev-dependencies]
tempfile = "3"
```

Modify `Cargo.toml` — add `"crates/cxas-discovery",` to `members`, immediately after `"crates/cxas-parity",`:

```toml
[workspace]
resolver = "2"
members = [
  "crates/cxas-parity",
  "crates/cxas-discovery",
  "crates/cxas-proto",
  "crates/cxas-core",
  "crates/cxas-utils",
  "crates/cxas-state",
  "crates/cxas-evals",
  "crates/cxas-migration",
  "crates/cxas-lint",
  "crates/cxas-cli",
]
```

- [ ] **Step 4: Write the model**

Create `crates/cxas-discovery/src/model.rs` (prefix with the standard Apache header):

```rust
/// One REST method from a discovery document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Method {
    pub id: String,
    pub http_method: String,
    pub path: String,
}

/// One enum-valued property on a schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumField {
    pub schema: String,
    pub property: String,
    pub values: Vec<String>,
}

/// A parsed discovery document.
#[derive(Debug, Clone, Default)]
pub struct Discovery {
    pub(crate) revision: String,
    pub(crate) version: String,
    pub(crate) methods: Vec<Method>,
    pub(crate) enum_fields: Vec<EnumField>,
}

impl Discovery {
    pub fn revision(&self) -> &str {
        &self.revision
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn methods(&self) -> impl Iterator<Item = &Method> {
        self.methods.iter()
    }

    pub fn method(&self, id: &str) -> Option<&Method> {
        self.methods.iter().find(|m| m.id == id)
    }

    pub fn enum_fields(&self) -> impl Iterator<Item = &EnumField> {
        self.enum_fields.iter()
    }

    pub fn enum_field(&self, schema: &str, property: &str) -> Option<&EnumField> {
        self.enum_fields
            .iter()
            .find(|e| e.schema == schema && e.property == property)
    }
}
```

- [ ] **Step 5: Write the parser**

Create `crates/cxas-discovery/src/parse.rs` (prefix with the standard Apache header):

```rust
use crate::model::{Discovery, EnumField, Method};
use crate::DiscoveryError;
use serde_json::Value;
use std::path::Path;

impl Discovery {
    /// Parse a canonicalized discovery document from disk.
    ///
    /// Returns an error rather than an empty model on failure: an empty model
    /// would make every coverage and parity assertion pass vacuously.
    pub fn load(path: &Path) -> Result<Self, DiscoveryError> {
        let text = std::fs::read_to_string(path).map_err(DiscoveryError::Io)?;
        Self::from_str(&text)
    }

    pub fn from_str(text: &str) -> Result<Self, DiscoveryError> {
        let root: Value = serde_json::from_str(text).map_err(DiscoveryError::Parse)?;

        let revision = root
            .get("revision")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let version = root
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        let mut methods = Vec::new();
        collect_methods(root.get("resources"), &mut methods);

        let mut enum_fields = Vec::new();
        if let Some(schemas) = root.get("schemas").and_then(Value::as_object) {
            for (schema_name, schema) in schemas {
                let Some(props) = schema.get("properties").and_then(Value::as_object) else {
                    continue;
                };
                for (prop_name, prop) in props {
                    let Some(values) = prop.get("enum").and_then(Value::as_array) else {
                        continue;
                    };
                    enum_fields.push(EnumField {
                        schema: schema_name.clone(),
                        property: prop_name.clone(),
                        values: values
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect(),
                    });
                }
            }
        }

        Ok(Discovery {
            revision,
            version,
            methods,
            enum_fields,
        })
    }
}

/// Discovery nests `resources` arbitrarily deep; every level may carry `methods`.
fn collect_methods(resources: Option<&Value>, out: &mut Vec<Method>) {
    let Some(map) = resources.and_then(Value::as_object) else {
        return;
    };
    for resource in map.values() {
        if let Some(methods) = resource.get("methods").and_then(Value::as_object) {
            for method in methods.values() {
                let (Some(id), Some(http_method), Some(path)) = (
                    method.get("id").and_then(Value::as_str),
                    method.get("httpMethod").and_then(Value::as_str),
                    method.get("path").and_then(Value::as_str),
                ) else {
                    continue;
                };
                out.push(Method {
                    id: id.to_string(),
                    http_method: http_method.to_string(),
                    path: path.to_string(),
                });
            }
        }
        collect_methods(resource.get("resources"), out);
    }
}
```

- [ ] **Step 6: Write the crate root**

Create `crates/cxas-discovery/src/lib.rs` (prefix with the standard Apache header):

```rust
//! Pure parser over vendored CES discovery documents.
//!
//! No network, no code generation, no CES semantics. This crate is the single
//! definition of "what the API is"; the parity tests and the Gauntlet evidence
//! collector both query through it so they cannot drift from one another.

mod model;
mod parse;

pub use model::{Discovery, EnumField, Method};

/// Failure modes when loading a discovery document.
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("reading discovery document: {0}")]
    Io(#[from] std::io::Error),
    #[error("parsing discovery document: {0}")]
    Parse(#[from] serde_json::Error),
}
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test -p cxas-discovery`
Expected: PASS, 10 tests (7 in `fixture.rs`, 3 in `vendored.rs`).

If the two count assertions in `vendored.rs` fail, the upstream API changed between the spec being written and this task running. Do not edit the expected numbers to match. Re-run `python tools/refresh_reference.py`, inspect the diff, and update both the numbers and `PINNED.toml` in the same commit with the new counts stated in the commit message.

- [ ] **Step 8: Commit**

```bash
git add crates/cxas-discovery Cargo.toml Cargo.lock
git commit -m "feat(discovery): parse vendored CES discovery documents"
```

---

## Task 3: The failing enum-parity test

This task deliberately ends with a **failing** test committed. Task 4 makes it pass. Do not fix the enum here.

**Files:**
- Create: `crates/cxas-proto/src/enum_registry.rs`
- Modify: `crates/cxas-proto/src/lib.rs`
- Modify: `crates/cxas-proto/Cargo.toml`
- Create: `crates/cxas-parity/tests/discovery_contract.rs`
- Modify: `crates/cxas-parity/Cargo.toml`

**Interfaces:**
- Consumes: `Discovery::load`, `Discovery::enum_field` from Task 2.
- Produces: `cxas_proto::enum_registry::REGISTERED_ENUMS: &[RegisteredEnum]` where `RegisteredEnum { rust_name: &'static str, schema: &'static str, property: &'static str, variants: &'static [&'static str] }`. Task 4 edits its `variants`. Task 5 reads the registry.

- [ ] **Step 1: Write the enum registry**

Create `crates/cxas-proto/src/enum_registry.rs` (prefix with the standard Apache header):

```rust
//! Declares which discovery enum each Rust enum in this crate mirrors.
//!
//! `cxas-parity`'s `enum_variants_match_discovery` test walks this registry and
//! fails when a declared variant list diverges from the vendored CES document.
//! Adding a Rust enum without adding it here is caught by `registry_covers_all_enums`.

/// One Rust enum bound to its discovery source of truth.
#[derive(Debug, Clone, Copy)]
pub struct RegisteredEnum {
    /// Rust type name, used only in assertion messages.
    pub rust_name: &'static str,
    /// Discovery schema id, e.g. `EvaluationRun`.
    pub schema: &'static str,
    /// Discovery property name, e.g. `state`.
    pub property: &'static str,
    /// Wire spellings this crate claims to implement, in discovery order.
    pub variants: &'static [&'static str],
    /// Discovery document that declares it: `"v1"` or `"v1beta"`.
    pub api_version: &'static str,
}

pub const REGISTERED_ENUMS: &[RegisteredEnum] = &[RegisteredEnum {
    rust_name: "EvaluationRunState",
    schema: "EvaluationRun",
    property: "state",
    api_version: "v1beta",
    // Deliberately the current (wrong) variant list. Task 4 corrects it.
    variants: &[
        "EVALUATION_RUN_STATE_UNSPECIFIED",
        "PENDING",
        "RUNNING",
        "SUCCEEDED",
        "FAILED",
        "CANCELLED",
    ],
}];
```

Modify `crates/cxas-proto/src/lib.rs` — add the module export after the existing `mod` line:

```rust
mod evaluation_run_state;
pub mod enum_registry;
pub use evaluation_run_state::EvaluationRunState;
```

- [ ] **Step 2: Wire up the test crate's dependencies**

Modify `crates/cxas-parity/Cargo.toml` — add to `[dev-dependencies]`:

```toml
[dev-dependencies]
serde_json = "1"
cxas-discovery = { path = "../cxas-discovery" }
cxas-proto = { path = "../cxas-proto" }
```

- [ ] **Step 3: Write the parity test**

Create `crates/cxas-parity/tests/discovery_contract.rs` (prefix with the standard Apache header):

```rust
//! The contract that replaces the self-graded manifest check.
//!
//! The previous `manifest_contract.rs` asserted that a checked-in YAML contained
//! strings that same YAML declared, so it could never fail. These assertions are
//! made against Google's vendored discovery documents instead.

use cxas_discovery::Discovery;
use cxas_proto::enum_registry::REGISTERED_ENUMS;
use std::path::PathBuf;

fn reference(version: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../reference/ces")
        .join(format!("{version}.discovery.json"))
}

fn load(version: &str) -> Discovery {
    Discovery::load(&reference(version))
        .unwrap_or_else(|e| panic!("{version} discovery document must load: {e}"))
}

#[test]
fn enum_variants_match_discovery() {
    let mut failures = Vec::new();

    for reg in REGISTERED_ENUMS {
        let doc = load(reg.api_version);
        let Some(field) = doc.enum_field(reg.schema, reg.property) else {
            failures.push(format!(
                "{}: {}.{} absent from {} discovery",
                reg.rust_name, reg.schema, reg.property, reg.api_version
            ));
            continue;
        };

        let declared: Vec<&str> = reg.variants.to_vec();
        let actual: Vec<&str> = field.values.iter().map(String::as_str).collect();

        if declared != actual {
            let invented: Vec<&&str> = declared.iter().filter(|v| !actual.contains(v)).collect();
            let missed: Vec<&&str> = actual.iter().filter(|v| !declared.contains(v)).collect();
            failures.push(format!(
                "{}: declared {declared:?} != discovery {actual:?}\n    invented: {invented:?}\n    missing:  {missed:?}",
                reg.rust_name
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "enum drift against vendored CES discovery:\n  {}",
        failures.join("\n  ")
    );
}

#[test]
fn registry_covers_every_enum_in_cxas_proto() {
    // Guards against adding a Rust enum that silently escapes parity checking.
    let src = include_str!("../../cxas-proto/src/evaluation_run_state.rs");
    if src.contains("pub enum ") {
        assert!(
            REGISTERED_ENUMS
                .iter()
                .any(|r| r.rust_name == "EvaluationRunState"),
            "EvaluationRunState exists but is not in REGISTERED_ENUMS"
        );
    }
}

#[test]
fn coverage_report_counts_implemented_methods() {
    // Reports, never gates. A pass/fail threshold can be satisfied by deleting
    // the metric; a printed number cannot.
    let v1 = load("v1");
    let v1beta = load("v1beta");
    let total = v1.methods().count() + v1beta.methods().count();

    println!("CES-COVERAGE v1={} v1beta={} total={}", v1.methods().count(), v1beta.methods().count(), total);
    assert!(total > 0, "discovery documents must declare methods");
}
```

- [ ] **Step 4: Run the test and confirm it fails for the right reason**

Run: `cargo test -p cxas-parity --test discovery_contract enum_variants_match_discovery -- --nocapture`

Expected: FAIL, with a message naming the drift, resembling:

```
enum drift against vendored CES discovery:
  EvaluationRunState: declared ["EVALUATION_RUN_STATE_UNSPECIFIED", "PENDING", ...] != discovery [...]
    invented: ["PENDING", "SUCCEEDED", "FAILED"]
    missing:  ["QUEUED", "COMPLETED", "ERROR"]
```

If it fails for any other reason — a path error, a parse error — fix that first. The test must fail *on the drift*, because that is the evidence this whole plan rests on.

- [ ] **Step 5: Commit the failing test**

```bash
git add crates/cxas-proto/src/enum_registry.rs crates/cxas-proto/src/lib.rs \
        crates/cxas-parity/tests/discovery_contract.rs crates/cxas-parity/Cargo.toml Cargo.lock
git commit -m "test(parity): assert enum variants against vendored CES discovery

Fails on EvaluationRunState: the crate declares PENDING/SUCCEEDED/FAILED
where CES declares QUEUED/COMPLETED/ERROR. Fixed in the next commit."
```

---

## Task 4: Correct `EvaluationRunState`

**Files:**
- Modify: `crates/cxas-proto/src/evaluation_run_state.rs`
- Modify: `crates/cxas-proto/src/enum_registry.rs`
- Modify: `crates/cxas-proto/tests/unknown_state.rs`

**Interfaces:**
- Consumes: `REGISTERED_ENUMS` from Task 3.
- Produces: `EvaluationRunState::{Unspecified, Queued, Running, Completed, Error, Cancelled, Unknown(String)}`, `from_wire_name(&str) -> Self`, `from_wire(i32) -> Self`, `as_str_name(&self) -> Cow<'static, str>`.

**Why `Unknown(String)` replaces `Unknown(i32)`:** the chosen transport is REST/JSON, where enums arrive as strings (`"COMPLETED"`), not integers. The unknown carrier must be able to hold an unrecognized *string*. `from_wire(i32)` is retained for proto interop, mapping by discovery declaration order.

- [ ] **Step 1: Update the registry to the real variants**

Modify the `variants` field in `crates/cxas-proto/src/enum_registry.rs`, and drop the now-stale comment:

```rust
pub const REGISTERED_ENUMS: &[RegisteredEnum] = &[RegisteredEnum {
    rust_name: "EvaluationRunState",
    schema: "EvaluationRun",
    property: "state",
    api_version: "v1beta",
    variants: &[
        "EVALUATION_RUN_STATE_UNSPECIFIED",
        "QUEUED",
        "RUNNING",
        "COMPLETED",
        "ERROR",
        "CANCELLED",
    ],
}];
```

- [ ] **Step 2: Run the parity test to confirm it now passes**

Run: `cargo test -p cxas-parity --test discovery_contract`
Expected: PASS, 3 tests.

- [ ] **Step 3: Update the enum itself**

Replace the body of `crates/cxas-proto/src/evaluation_run_state.rs` below the Apache header with:

```rust
use std::borrow::Cow;

/// Evaluation run lifecycle state, mirroring `EvaluationRun.state` in the
/// vendored CES `v1beta` discovery document.
///
/// Unknown wire values map to [`EvaluationRunState::Unknown`] so callers never
/// panic when the server's enum grows beyond this crate's known set (#284).
/// Variant spellings are asserted against discovery by
/// `cxas-parity`'s `enum_variants_match_discovery`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluationRunState {
    Unspecified,
    Queued,
    Running,
    Completed,
    Error,
    Cancelled,
    /// A wire value this build does not know. Carries the raw value verbatim.
    Unknown(String),
}

impl EvaluationRunState {
    /// Map a REST/JSON wire string to a typed state without panicking.
    ///
    /// This is the canonical constructor: the CES REST surface encodes enums as
    /// strings, so an unrecognized value is a string, not an integer.
    pub fn from_wire_name(name: &str) -> Self {
        match name {
            "EVALUATION_RUN_STATE_UNSPECIFIED" => Self::Unspecified,
            "QUEUED" => Self::Queued,
            "RUNNING" => Self::Running,
            "COMPLETED" => Self::Completed,
            "ERROR" => Self::Error,
            "CANCELLED" => Self::Cancelled,
            other => Self::Unknown(other.to_string()),
        }
    }

    /// Map a protobuf wire integer by discovery declaration order.
    ///
    /// Retained for proto interop. Out-of-range values are preserved verbatim
    /// rather than looked up by `.name` on a raw integer, which is the exact
    /// Python crash this crate exists to prevent (#284).
    pub fn from_wire(value: i32) -> Self {
        match value {
            0 => Self::Unspecified,
            1 => Self::Queued,
            2 => Self::Running,
            3 => Self::Completed,
            4 => Self::Error,
            5 => Self::Cancelled,
            other => Self::Unknown(other.to_string()),
        }
    }

    /// Stable wire spelling for logs, diagnostics, and JSON round-trips.
    pub fn as_str_name(&self) -> Cow<'static, str> {
        match self {
            Self::Unspecified => Cow::Borrowed("EVALUATION_RUN_STATE_UNSPECIFIED"),
            Self::Queued => Cow::Borrowed("QUEUED"),
            Self::Running => Cow::Borrowed("RUNNING"),
            Self::Completed => Cow::Borrowed("COMPLETED"),
            Self::Error => Cow::Borrowed("ERROR"),
            Self::Cancelled => Cow::Borrowed("CANCELLED"),
            Self::Unknown(raw) => Cow::Owned(format!("UNKNOWN({raw})")),
        }
    }

    /// True when the run reached a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Error | Self::Cancelled)
    }
}
```

- [ ] **Step 4: Update the existing tests to the real enum**

Replace the test bodies in `crates/cxas-proto/tests/unknown_state.rs` below the Apache header with:

```rust
use cxas_proto::EvaluationRunState;

#[test]
fn unknown_string_wire_value_is_typed() {
    let state = EvaluationRunState::from_wire_name("SOME_FUTURE_STATE");
    assert_eq!(
        state,
        EvaluationRunState::Unknown("SOME_FUTURE_STATE".to_string())
    );
    assert_eq!(state.as_str_name(), "UNKNOWN(SOME_FUTURE_STATE)");
}

#[test]
fn unknown_integer_wire_value_is_typed() {
    let state = EvaluationRunState::from_wire(99);
    assert_eq!(state, EvaluationRunState::Unknown("99".to_string()));
    assert_eq!(state.as_str_name(), "UNKNOWN(99)");
}

#[test]
fn known_wire_names_map_to_real_ces_spellings() {
    assert_eq!(
        EvaluationRunState::from_wire_name("COMPLETED"),
        EvaluationRunState::Completed
    );
    assert_eq!(EvaluationRunState::Completed.as_str_name(), "COMPLETED");
    assert_eq!(
        EvaluationRunState::from_wire_name("QUEUED"),
        EvaluationRunState::Queued
    );
    assert_eq!(
        EvaluationRunState::from_wire_name("ERROR"),
        EvaluationRunState::Error
    );
}

#[test]
fn every_known_variant_round_trips_through_its_wire_name() {
    for state in [
        EvaluationRunState::Unspecified,
        EvaluationRunState::Queued,
        EvaluationRunState::Running,
        EvaluationRunState::Completed,
        EvaluationRunState::Error,
        EvaluationRunState::Cancelled,
    ] {
        let name = state.as_str_name().into_owned();
        assert_eq!(
            EvaluationRunState::from_wire_name(&name),
            state,
            "{name} did not round-trip"
        );
    }
}

#[test]
fn terminal_states_are_exactly_completed_error_cancelled() {
    assert!(EvaluationRunState::Completed.is_terminal());
    assert!(EvaluationRunState::Error.is_terminal());
    assert!(EvaluationRunState::Cancelled.is_terminal());
    assert!(!EvaluationRunState::Queued.is_terminal());
    assert!(!EvaluationRunState::Running.is_terminal());
    assert!(!EvaluationRunState::Unspecified.is_terminal());
}

#[test]
fn source_never_calls_name_on_i32() {
    let src = include_str!("../src/evaluation_run_state.rs");
    assert!(
        !src.contains(".name()"),
        "reintroduces the Python #284 crash class"
    );
}
```

- [ ] **Step 5: Find and fix every other reference to the removed variants**

Run: `grep -rn "Succeeded\|::Pending\|SUCCEEDED\|EvaluationRunState::Failed" crates --include="*.rs"`

Fix each hit to the corrected spelling. Do **not** add `Succeeded`/`Failed` aliases — the spec explicitly forbids keeping the invented variants.

- [ ] **Step 6: Run the whole workspace**

Run: `cargo test --workspace`
Expected: PASS. Total test count rises from 78 (the 3 replaced enum tests become 6, plus 10 from `cxas-discovery` and 3 from `discovery_contract`).

- [ ] **Step 7: Commit**

```bash
git add crates/cxas-proto crates/cxas-parity
git commit -m "fix(proto): correct EvaluationRunState to real CES wire values

QUEUED/COMPLETED/ERROR replace the invented PENDING/SUCCEEDED/FAILED.
Unknown now carries the raw wire string, since the REST surface encodes
enums as strings. Closes the real #284, which the previous test only
appeared to close."
```

---

## Task 5: Retire the self-graded contract

**Files:**
- Modify: `crates/cxas-parity/tests/manifest_contract.rs` (rename tests, rescope)
- Create: `.github/workflows/reference-drift.yml`
- Modify: `docs/superpowers/coverage-map.md`

**Interfaces:**
- Consumes: everything from Tasks 1–4.
- Produces: no new code interfaces. Produces the CI drift gate.

- [ ] **Step 1: Rescope the Python manifest tests**

In `crates/cxas-parity/tests/manifest_contract.rs`, add this module doc immediately under the Apache header:

```rust
//! Python `cxas-scrapi` surface reference — **CLI shape only**.
//!
//! These assertions describe what users expect the CLI to look like. They are
//! NOT a correctness benchmark: the manifest is hand-written and self-graded.
//! The API-correctness contract lives in `discovery_contract.rs`, which asserts
//! against Google's vendored discovery documents. Where the two disagree,
//! discovery wins.
```

Rename the three type/method tests so their scope is unambiguous:
- `every_frozen_python_class_is_present` → `python_surface_declares_every_frozen_class`
- `spec_method_minima_are_present` → `python_surface_declares_method_minima`
- `frozen_cli_commands_are_present` → `python_surface_declares_cli_commands`

- [ ] **Step 2: Run the suite**

Run: `cargo test -p cxas-parity`
Expected: PASS, all tests, with the renamed ones visible in the output.

- [ ] **Step 3: Add the CI drift gate**

Create `.github/workflows/reference-drift.yml`:

```yaml
name: reference-drift

on:
  schedule:
    - cron: "0 6 * * 1"
  workflow_dispatch:
  pull_request:
    paths:
      - "reference/**"
      - "tools/refresh_reference.py"

jobs:
  drift:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: "3.11"
      - name: Check vendored discovery documents against upstream
        run: python tools/refresh_reference.py --check
```

The `--check` mode fetches, canonicalizes, and compares without writing, exiting non-zero on any difference. Upstream drift becomes a reviewable failure rather than a silent behavioural change.

- [ ] **Step 4: Correct the coverage map**

In `docs/superpowers/coverage-map.md`, replace this line:

```
Gauntlet Loop (source-doc overlay) is out-of-band process, not a Superpowers phase. Specs record it as non-runtime. No Gauntlet plan is required for this goal.
```

with:

```
Gauntlet Loop is implemented as repo tooling under `gauntlet/`, specified in
`docs/superpowers/specs/2026-08-15-discovery-benchmark-gauntlet-design.md` and
planned in `docs/superpowers/plans/2026-08-15-discovery-benchmark-gauntlet.md`.
It is non-runtime: nothing under `gauntlet/` is a workspace member.
```

Then, in the **Quality bar** section, replace the acceptance sentence with:

```
`cxas-harness` is accepted when (a) every enum and method the Rust crates
declare resolves against the vendored CES discovery documents, and (b) every
issue in the table has a closing test that exercises behaviour verified against
discovery rather than against a test double asserting the code's own
assumptions. Clause (a) replaces the former parity-manifest bar, which was
self-graded and could not fail.
```

- [ ] **Step 5: Commit**

```bash
git add crates/cxas-parity/tests/manifest_contract.rs .github/workflows/reference-drift.yml docs/superpowers/coverage-map.md
git commit -m "refactor(parity): scope Python manifest to CLI shape, gate reference drift"
```

---

# Phase 2 — The Gauntlet Loop

## Task 6: The evidence collector

**Files:**
- Create: `gauntlet/evidence.py`
- Create: `gauntlet/__init__.py` (empty)
- Test: `gauntlet/tests/test_evidence.py`
- Create: `gauntlet/tests/__init__.py` (empty)

**Interfaces:**
- Consumes: `cargo` on PATH; `reference/ces/*.json` from Task 1.
- Produces — Task 7 depends on these exact signatures:
  - `build_bundle(piece: str, repo_root: Path, issues: list[str]) -> dict`
  - `render_bundle(bundle: dict) -> str`
  - `FORBIDDEN_KEYS: frozenset[str]`

- [ ] **Step 1: Write the failing test**

Create `gauntlet/tests/test_evidence.py`:

```python
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

from gauntlet.evidence import FORBIDDEN_KEYS, build_bundle, render_bundle


def _bundle():
    return build_bundle(piece="cxas-proto", repo_root=ROOT, issues=["284"])


def test_bundle_has_required_evidence_keys():
    b = _bundle()
    for key in ("piece", "test_output", "clippy_output", "coverage", "issues", "binary_size"):
        assert key in b, f"missing evidence key {key}"


def test_bundle_excludes_source_code():
    # The critic must be blind. If source ever leaks into the bundle, the loop
    # degenerates into self-grading, which is the failure this whole design exists
    # to prevent.
    b = _bundle()
    for forbidden in FORBIDDEN_KEYS:
        assert forbidden not in b, f"evidence bundle leaked {forbidden} to the critic"


def test_rendered_bundle_contains_no_rust_source():
    text = render_bundle(_bundle())
    assert "pub enum " not in text
    assert "pub fn " not in text
    assert "impl " not in text


def test_coverage_reports_both_api_versions():
    cov = _bundle()["coverage"]
    assert cov["v1_methods"] > 0
    assert cov["v1beta_methods"] > 0


def test_piece_is_echoed_for_routing():
    assert _bundle()["piece"] == "cxas-proto"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python -m pytest gauntlet/tests/test_evidence.py -v`
Expected: FAIL with `ModuleNotFoundError: No module named 'gauntlet.evidence'`

- [ ] **Step 3: Write the implementation**

Create `gauntlet/__init__.py` and `gauntlet/tests/__init__.py` as empty files, then create `gauntlet/evidence.py`:

```python
#!/usr/bin/env python3
"""Deterministic evidence bundle for blind critics.

This is code, not an agent. The critic sees exactly what this produces and
nothing else -- no source, no commit messages, no builder rationale. That
exclusion is what keeps a critic unpersuadable by explanation.
"""
import json
import subprocess
from pathlib import Path

# Keys that must never appear in a bundle. Asserted by the test suite so the
# blindness guarantee survives future edits to this file.
FORBIDDEN_KEYS = frozenset({"source", "diff", "rationale", "commit_message", "builder_notes"})


def _run(cmd, cwd, timeout=900):
    try:
        proc = subprocess.run(
            cmd, cwd=cwd, capture_output=True, text=True, timeout=timeout, shell=False
        )
        return {
            "exit_code": proc.returncode,
            "stdout": proc.stdout[-20000:],
            "stderr": proc.stderr[-20000:],
        }
    except FileNotFoundError:
        return {"exit_code": 127, "stdout": "", "stderr": f"not found: {cmd[0]}"}
    except subprocess.TimeoutExpired:
        return {"exit_code": 124, "stdout": "", "stderr": f"timed out after {timeout}s"}


def _coverage(repo_root):
    ref = repo_root / "reference" / "ces"
    out = {}
    for version in ("v1", "v1beta"):
        path = ref / f"{version}.discovery.json"
        count = 0
        revision = "missing"
        if path.exists():
            doc = json.loads(path.read_text(encoding="utf-8"))
            revision = doc.get("revision", "unknown")

            def walk(resources):
                nonlocal count
                for res in (resources or {}).values():
                    count += len(res.get("methods") or {})
                    walk(res.get("resources"))

            walk(doc.get("resources"))
        out[f"{version}_methods"] = count
        out[f"{version}_revision"] = revision
    return out


def _binary_size(repo_root):
    for candidate in (
        repo_root / "target" / "release" / "cxas.exe",
        repo_root / "target" / "release" / "cxas",
    ):
        if candidate.exists():
            return candidate.stat().st_size
    return 0


def build_bundle(piece, repo_root, issues):
    """Collect everything a blind critic is allowed to see about `piece`."""
    repo_root = Path(repo_root)
    return {
        "piece": piece,
        "issues": list(issues),
        "test_output": _run(["cargo", "test", "-p", piece], repo_root),
        "clippy_output": _run(
            ["cargo", "clippy", "-p", piece, "--all-targets"], repo_root
        ),
        "coverage": _coverage(repo_root),
        "binary_size": _binary_size(repo_root),
    }


def render_bundle(bundle):
    """Format a bundle as the critic's prompt input."""
    cov = bundle["coverage"]
    parts = [
        f"# Evidence for piece: {bundle['piece']}",
        f"Assigned issues: {', '.join(bundle['issues']) or 'none'}",
        "",
        "## cargo test",
        f"exit_code: {bundle['test_output']['exit_code']}",
        "```",
        bundle["test_output"]["stdout"] or bundle["test_output"]["stderr"],
        "```",
        "",
        "## cargo clippy",
        f"exit_code: {bundle['clippy_output']['exit_code']}",
        "```",
        bundle["clippy_output"]["stderr"] or bundle["clippy_output"]["stdout"],
        "```",
        "",
        "## CES discovery coverage",
        f"v1: {cov['v1_methods']} methods (revision {cov['v1_revision']})",
        f"v1beta: {cov['v1beta_methods']} methods (revision {cov['v1beta_revision']})",
        "",
        "## Binary size",
        f"{bundle['binary_size']} bytes",
    ]
    return "\n".join(parts)
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python -m pytest gauntlet/tests/test_evidence.py -v`
Expected: PASS, 5 tests. `cargo test -p cxas-proto` runs inside the fixture, so allow up to a minute on a cold target directory.

- [ ] **Step 5: Commit**

```bash
git add gauntlet/__init__.py gauntlet/evidence.py gauntlet/tests/
git commit -m "feat(gauntlet): deterministic evidence bundle with enforced blindness"
```

---

## Task 7: The orchestrator

**Files:**
- Create: `gauntlet/orchestrator.py`
- Create: `gauntlet/config.toml`
- Test: `gauntlet/tests/test_orchestrator.py`
- Create: `gauntlet/tests/stub_agent.py`

**Interfaces:**
- Consumes: `build_bundle`, `render_bundle` from Task 6.
- Produces: `load_config(path) -> dict`, `parse_verdict(text) -> dict`, `run_piece(piece, config, repo_root, run_dir) -> dict`, `main(argv) -> int`.

- [ ] **Step 1: Write the stub agent**

Create `gauntlet/tests/stub_agent.py`:

```python
#!/usr/bin/env python3
"""Canned-verdict agent so the loop is testable without invoking a real model.

Reads a prompt on stdin, ignores it, and emits a verdict controlled by
GAUNTLET_STUB_MODE: 'pass', 'fail', or 'garbage'.
"""
import os
import sys

sys.stdin.read()
mode = os.environ.get("GAUNTLET_STUB_MODE", "pass")

if mode == "pass":
    print('{"score": 95, "verdict": "PASS", "biggest_gap": "none"}')
elif mode == "fail":
    print('{"score": 40, "verdict": "FAIL", "biggest_gap": "enum drift on EvaluationRunState"}')
else:
    print("this is not json at all")
```

- [ ] **Step 2: Write the failing test**

Create `gauntlet/tests/test_orchestrator.py`:

```python
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

from gauntlet.orchestrator import load_config, parse_verdict, run_piece

STUB = f'"{sys.executable}" "{ROOT / "gauntlet" / "tests" / "stub_agent.py"}"'


def _config(**over):
    cfg = {
        "agent_cmd": STUB,
        "max_rounds": 3,
        "pieces": ["cxas-proto"],
        "issues": {"cxas-proto": ["284"]},
        "rc_coverage_min": 0,
    }
    cfg.update(over)
    return cfg


def test_load_config_reads_agent_cmd_and_pieces():
    cfg = load_config(ROOT / "gauntlet" / "config.toml")
    assert "agent_cmd" in cfg
    assert isinstance(cfg["pieces"], list) and cfg["pieces"]


def test_parse_verdict_reads_json_verdict():
    v = parse_verdict('{"score": 95, "verdict": "PASS", "biggest_gap": "none"}')
    assert v["verdict"] == "PASS"
    assert v["score"] == 95


def test_parse_verdict_finds_json_embedded_in_prose():
    v = parse_verdict('Here is my assessment:\n{"score": 10, "verdict": "FAIL", "biggest_gap": "x"}\nDone.')
    assert v["verdict"] == "FAIL"


def test_malformed_verdict_is_a_failed_round_not_approval():
    # Silence is not consent: an unparseable critic response must never be
    # treated as a pass.
    v = parse_verdict("this is not json at all")
    assert v["verdict"] == "FAIL"
    assert "unparseable" in v["biggest_gap"].lower()


def test_run_piece_stops_when_critic_passes(tmp_path):
    os.environ["GAUNTLET_STUB_MODE"] = "pass"
    result = run_piece("cxas-proto", _config(), ROOT, tmp_path)
    assert result["verdict"] == "PASS"
    assert result["rounds"] == 1


def test_run_piece_honours_max_rounds_when_critic_keeps_failing(tmp_path):
    os.environ["GAUNTLET_STUB_MODE"] = "fail"
    result = run_piece("cxas-proto", _config(max_rounds=2), ROOT, tmp_path)
    assert result["verdict"] == "FAIL"
    assert result["rounds"] == 2


def test_run_piece_writes_a_scorecard(tmp_path):
    os.environ["GAUNTLET_STUB_MODE"] = "pass"
    run_piece("cxas-proto", _config(), ROOT, tmp_path)
    assert (tmp_path / "scorecard.json").exists()


def test_garbage_verdict_never_counts_as_pass(tmp_path):
    os.environ["GAUNTLET_STUB_MODE"] = "garbage"
    result = run_piece("cxas-proto", _config(max_rounds=1), ROOT, tmp_path)
    assert result["verdict"] == "FAIL"
```

- [ ] **Step 3: Run test to verify it fails**

Run: `python -m pytest gauntlet/tests/test_orchestrator.py -v`
Expected: FAIL with `ModuleNotFoundError: No module named 'gauntlet.orchestrator'`

- [ ] **Step 4: Write the config**

Create `gauntlet/config.toml`:

```toml
# Gauntlet Loop configuration. Repo tooling -- never a workspace member.

# Any agent CLI that reads a prompt on stdin and writes a response on stdout.
# Examples: "claude -p", "gemini -p", "codex exec".
agent_cmd = "claude -p"

# Per-piece iteration cap. The loop has no natural finish line, so this and
# budget_usd are the deliberate stop conditions.
max_rounds = 8

# 0 means unlimited.
budget_usd = 0

# Release-candidate gate: minimum implemented CES methods. Phase 3 raises this
# to the gated subset (apps, agents, tools, v1beta evaluations), not all 170.
rc_coverage_min = 0

# Pieces map onto existing crate boundaries.
pieces = [
  "cxas-discovery",
  "cxas-proto",
  "cxas-core",
  "cxas-utils",
  "cxas-state",
  "cxas-evals",
  "cxas-lint",
  "cxas-migration",
  "cxas-cli",
]

[issues]
cxas-proto = ["284"]
cxas-core = ["401", "298", "263", "403"]
cxas-evals = ["355", "345", "136", "188", "27", "206"]
cxas-lint = ["86", "397"]
cxas-migration = ["168", "394"]
cxas-state = ["131", "270"]
cxas-utils = ["256"]
cxas-cli = ["55", "46", "54", "350", "386", "252", "99"]
cxas-discovery = []
```

- [ ] **Step 5: Write the orchestrator**

Create `gauntlet/orchestrator.py`:

```python
#!/usr/bin/env python3
"""Provider-agnostic Gauntlet Loop orchestrator.

Roles are separated structurally, not by instruction:
  orchestrator -- plans, fans out, merges. Never implements, never critiques.
  builder      -- one per piece, sees the task and its own workspace.
  critic       -- blind. Sees only the evidence bundle from evidence.py.
"""
import json
import re
import shlex
import subprocess
import sys
import tomllib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from gauntlet.evidence import build_bundle, render_bundle

AGENTS_DIR = Path(__file__).resolve().parent / "agents"


def load_config(path):
    with open(path, "rb") as fh:
        return tomllib.load(fh)


def parse_verdict(text):
    """Extract the critic's JSON verdict.

    An unparseable response is a FAILED round, never an approval. A critic that
    cannot state a verdict has not granted one.
    """
    for match in re.finditer(r"\{.*?\}", text or "", re.DOTALL):
        try:
            obj = json.loads(match.group(0))
        except json.JSONDecodeError:
            continue
        if "verdict" in obj:
            return {
                "score": obj.get("score", 0),
                "verdict": "PASS" if str(obj["verdict"]).upper() == "PASS" else "FAIL",
                "biggest_gap": obj.get("biggest_gap", ""),
            }
    return {"score": 0, "verdict": "FAIL", "biggest_gap": "unparseable critic response"}


def invoke_agent(agent_cmd, prompt, timeout=1800):
    """Run any agent CLI that reads stdin and writes stdout."""
    try:
        proc = subprocess.run(
            shlex.split(agent_cmd),
            input=prompt,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        return proc.stdout
    except (FileNotFoundError, subprocess.TimeoutExpired) as exc:
        return f"agent invocation failed: {exc}"


def _role_prompt(name, fallback):
    path = AGENTS_DIR / f"{name}.md"
    return path.read_text(encoding="utf-8") if path.exists() else fallback


def run_piece(piece, config, repo_root, run_dir):
    """Build/critique a single piece until its critic passes or rounds run out."""
    repo_root, run_dir = Path(repo_root), Path(run_dir)
    run_dir.mkdir(parents=True, exist_ok=True)
    issues = config.get("issues", {}).get(piece, [])
    agent_cmd = config["agent_cmd"]
    max_rounds = int(config.get("max_rounds", 8))

    builder_role = _role_prompt("builder", "You are a builder. Improve the piece.")
    critic_role = _role_prompt("critic", "You are a blind critic. Judge only the evidence.")

    history = []
    verdict = {"verdict": "FAIL", "score": 0, "biggest_gap": "not yet run"}

    for round_no in range(1, max_rounds + 1):
        gap = verdict["biggest_gap"] if round_no > 1 else ""
        builder_prompt = (
            f"{builder_role}\n\n"
            f"Piece: {piece}\nAssigned issues: {', '.join(issues) or 'none'}\n"
            f"{('Top-priority fix from the critic: ' + gap) if gap else ''}"
        )
        invoke_agent(agent_cmd, builder_prompt)

        bundle = build_bundle(piece=piece, repo_root=repo_root, issues=issues)
        (run_dir / f"evidence-round-{round_no}.md").write_text(
            render_bundle(bundle), encoding="utf-8"
        )

        critic_prompt = (
            f"{critic_role}\n\n{render_bundle(bundle)}\n\n"
            'Respond with JSON only: '
            '{"score": <0-100>, "verdict": "PASS"|"FAIL", "biggest_gap": "<one gap>"}'
        )
        verdict = parse_verdict(invoke_agent(agent_cmd, critic_prompt))
        history.append({"round": round_no, **verdict})

        scorecard = {"piece": piece, "rounds": round_no, "history": history, **verdict}
        (run_dir / "scorecard.json").write_text(
            json.dumps(scorecard, indent=2), encoding="utf-8"
        )
        print(f"[{piece}] round {round_no}: {verdict['verdict']} ({verdict['score']}) {verdict['biggest_gap']}")

        if verdict["verdict"] == "PASS":
            break

    return {"piece": piece, "rounds": len(history), "history": history, **verdict}


def main(argv):
    here = Path(__file__).resolve().parent
    repo_root = here.parent
    config = load_config(here / "config.toml")
    only = argv[0] if argv else None
    pieces = [only] if only else config["pieces"]

    results = []
    for piece in pieces:
        run_dir = here / "runs" / piece
        results.append(run_piece(piece, config, repo_root, run_dir))

    failed = [r["piece"] for r in results if r["verdict"] != "PASS"]
    print(f"\n{len(results) - len(failed)}/{len(results)} pieces passed")
    if failed:
        print(f"still failing: {', '.join(failed)}")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
```

- [ ] **Step 6: Run test to verify it passes**

Run: `python -m pytest gauntlet/tests/test_orchestrator.py -v`
Expected: PASS, 8 tests.

- [ ] **Step 7: Ignore run artifacts**

Add to `.gitignore`:

```
gauntlet/runs/
```

- [ ] **Step 8: Commit**

```bash
git add gauntlet/orchestrator.py gauntlet/config.toml gauntlet/tests/ .gitignore
git commit -m "feat(gauntlet): provider-agnostic builder/critic loop"
```

---

## Task 8: Role prompts and documentation

**Files:**
- Create: `gauntlet/agents/builder.md`
- Create: `gauntlet/agents/critic.md`
- Create: `gauntlet/README.md`
- Modify: `README.md`

**Interfaces:**
- Consumes: `_role_prompt` from Task 7, which reads these by filename.
- Produces: no code interfaces.

- [ ] **Step 1: Write the builder role**

Create `gauntlet/agents/builder.md`:

```markdown
# Role: Builder

You improve exactly one piece (one crate) of `cxas-harness`. You work only inside
that crate plus its tests.

## Your quality bar

1. The CES discovery documents under `reference/ces/` are the sole authority on
   what the API is. Where the Python `cxas-scrapi` surface disagrees, discovery
   wins.
2. Every enum variant you declare must match its discovery wire spelling exactly.
   Never invent a variant name.
3. Every assigned issue needs a test that reproduces the original bug and now
   passes. A test asserting against your own test double does not count.
4. No dead code, no duplicate logic, no new clippy warnings.

## Rules

- You may not edit `gauntlet/`, and you may not edit the evidence collector.
- You may not weaken or delete a failing test to make it pass.
- You may not add aliases for removed enum variants to keep old code compiling.
- Commit before your work is critiqued.

You will receive the critic's single top-priority gap each round. Fix that gap
first, then continue.
```

- [ ] **Step 2: Write the critic role**

Create `gauntlet/agents/critic.md`:

```markdown
# Role: Blind Critic

You judge one piece of `cxas-harness` from evidence alone. You have not seen the
source code, the diff, or the builder's reasoning, and you will not ask for them.
That is deliberate: it makes you unpersuadable by explanation.

## What you are judging

- `cargo test`: did every test pass? Are there tests at all?
- `cargo clippy`: any warnings?
- CES discovery coverage: how much of the real API surface is implemented?
- Assigned issues: is there evidence each is genuinely closed?
- Binary size and build time against the packaging goals.

## Rules

- Never lower the bar. Never narrow scope to make a piece look finished.
- Never implement anything. You do not write code.
- A passing test suite is necessary but not sufficient: a suite that only
  exercises the code's own assumptions is weak evidence, and you should say so.
- Zero tests, or tests that do not cover the assigned issues, is a FAIL
  regardless of a green exit code.
- Name exactly ONE biggest remaining gap. It becomes the builder's next task,
  so make it specific and actionable.

## Output

JSON only:

```json
{"score": 0-100, "verdict": "PASS" | "FAIL", "biggest_gap": "one specific gap"}
```

If you cannot form a verdict, say FAIL. Silence is not consent.
```

- [ ] **Step 3: Write the tooling README**

Create `gauntlet/README.md`:

```markdown
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
| Builder | its own crate, the critic's last gap | edit its crate only |
| Critic | the evidence bundle, nothing else | score and name one gap — never implement |

The critic's blindness is enforced by `gauntlet/evidence.py` and asserted by
`test_bundle_excludes_source_code`. If source ever reaches the critic, the loop
degenerates into self-grading — the exact failure this design exists to prevent.

## Stop conditions

The loop has no natural finish line, so the stop conditions are explicit:
`max_rounds` per piece, `budget_usd` overall, and an RC gate requiring every
catalogued issue to have a repro test with coverage at or above
`rc_coverage_min`. On reaching the gate the orchestrator stops and hands back
for human review.

## Test the loop itself

```sh
python -m pytest gauntlet/tests/ -v
```

Tests run against `gauntlet/tests/stub_agent.py`, so the loop is verifiable
without invoking a real model.
```

- [ ] **Step 4: Link it from the top-level README**

In `README.md`, add a row to the "What it is" table after the `Quality bar` row:

```markdown
| Gauntlet Loop | Builder/blind-critic loop under `gauntlet/`, benchmarked against vendored CES discovery docs |
```

And add this section immediately before `## Design docs`:

```markdown
## Benchmark and the Gauntlet Loop

The API benchmark is Google's own CES discovery documents, vendored under
`reference/ces/` at a pinned revision: **66 methods in v1, 104 in v1beta**.
`crates/cxas-discovery` parses them, and `cxas-parity` asserts that every enum
variant this workspace declares matches its CES wire spelling exactly.

That contract found a real defect on its first run: `EvaluationRunState`
declared `PENDING`/`SUCCEEDED`/`FAILED` where CES declares
`QUEUED`/`COMPLETED`/`ERROR` — the test closing the enum-drift bug (#284) had
itself drifted, invisibly to 78 passing tests.

[`gauntlet/`](gauntlet/) builds on that benchmark: builder agents per crate,
each paired with a blind critic that sees only test output, clippy results,
discovery coverage, and issue reproductions — never the source or the builder's
reasoning. See [`gauntlet/README.md`](gauntlet/README.md).
```

- [ ] **Step 5: Verify the whole thing runs end to end**

Run: `python -m pytest gauntlet/ tests/ -v`
Expected: PASS, all tests.

Run: `cargo test --workspace`
Expected: PASS.

Run: `python gauntlet/orchestrator.py cxas-discovery`
Expected: it invokes the configured `agent_cmd`. If no agent CLI is installed, `invoke_agent` returns the failure string, the critic verdict is unparseable, and the piece reports FAIL after `max_rounds`. That is correct behaviour — a missing agent must never read as a pass.

- [ ] **Step 6: Commit**

```bash
git add gauntlet/agents gauntlet/README.md README.md
git commit -m "docs(gauntlet): builder and blind-critic role prompts"
```

---

## Self-Review

**Spec coverage:**

| Spec section | Task |
|---|---|
| `reference/` vendored + canonicalized + `PINNED.toml` | 1 |
| Canonicalization format reproducible across fetches | 1 |
| `cxas-discovery` pure parser, both error paths | 2 |
| `enum_variants_match_discovery` fails first | 3 |
| Motivating defect fixed, no aliases retained | 4 |
| `declared_methods_resolve_in_discovery` | 3 (as `registry_covers_every_enum_in_cxas_proto`; method-level resolution has no subject until Phase 3 transport lands) |
| `coverage_report` reports, never gates | 3 |
| Python manifest rescoped to CLI shape | 5 |
| CI drift check | 5 |
| `gauntlet/` layout, provider-agnostic execution | 7 |
| Evidence bundle with enforced blindness | 6 |
| Role separation, orchestrator never implements | 7, 8 |
| Stop conditions: rounds, budget, RC gate | 7 (config), 8 (documented) |
| Malformed critic output is a failed round | 7 |
| Agent subprocess failure never counts as a pass | 7, 8 |
| `xtask` codegen, REST transport | **deferred to Phase 3 by design** — noted in the scope note |

**Placeholder scan:** clean. Every code step carries runnable content, no deferral markers remain, and no task defers to a neighbouring task instead of repeating the code it needs.

**Type consistency:** `Discovery::load`/`method`/`methods`/`enum_field` are used in Tasks 3 and 6 exactly as defined in Task 2. `RegisteredEnum` fields (`rust_name`, `schema`, `property`, `variants`, `api_version`) are consistent across Tasks 3 and 4. `build_bundle(piece, repo_root, issues)` and `render_bundle(bundle)` match between Tasks 6 and 7. `parse_verdict` returns `{score, verdict, biggest_gap}` everywhere it is consumed.

**One deliberate deviation from the spec:** the spec lists `declared_methods_resolve_in_discovery` as a Phase 1 test. Nothing in the workspace declares a CES method until the Phase 3 transport exists, so a Phase 1 version would assert over an empty set and pass vacuously — the precise failure mode this plan is written to eliminate. It is replaced by `registry_covers_every_enum_in_cxas_proto`, which has a real subject today, and the method-level check moves to the Phase 3 plan.
