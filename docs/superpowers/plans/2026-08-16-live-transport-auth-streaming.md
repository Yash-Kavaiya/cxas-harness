# Live Transport, Credentials, and Streaming Implementation Plan

**Goal:** Make `cxas-harness` a CES client rather than a CES describer — every
method addressable, credentials resolved the way Google resolves them, and
`streamRunSession` delivered incrementally — without any test needing a cloud
project.

**Architecture:** Generated method table from the vendored discovery documents;
pure request construction, credential resolution, and stream decoding; a thin
HTTP layer behind the `rest` feature; a generic `cxas api` CLI surface behind
the `remote` feature.

**Tech Stack:** Rust 2021 (MSRV 1.80), `reqwest` (rustls), `tokio`,
`serde_json`, `clap` 4, Python 3.11 for the generator and its tests.

**Spec:** docs/superpowers/specs/2026-08-16-live-transport-auth-streaming-design.md

## Global Constraints

- The vendored reference is the only source of truth. No path template, verb,
  or method id is written by hand.
- Anything that can be pure, is: request construction, credential resolution,
  and stream decoding do no I/O and compile without the `rest` feature.
- No test may require a network, a credential, or a cloud project.
- An unusable credential is an error at its own precedence, never a silent
  fallback to a different identity.
- Coverage is reported as two numbers — addressable and modelled — and they are
  never collapsed into one.
- `cargo clippy --workspace --all-targets` stays clean.

## File Structure

```text
tools/generate_methods.py                          new  generator + --check
tests/test_generate_methods.py                     new  generator tests
crates/cxas-core/src/rest/method_table.rs          new  GENERATED, 170 specs
crates/cxas-core/src/rest/method.rs                edit types, MODELLED, helpers
crates/cxas-core/src/rest/stream.rs                new  JsonStreamDecoder
crates/cxas-core/src/rest/http.rs                  edit stream(), token wiring
crates/cxas-core/src/auth.rs                       new  ADC resolution + provider
crates/cxas-core/src/error.rs                      edit CoreError::Auth
crates/cxas-core/tests/support/mod.rs              new  scripted loopback stub
crates/cxas-core/tests/auth_resolve.rs             new  precedence, pure
crates/cxas-core/tests/auth_provider.rs            new  minting, stub server
crates/cxas-core/tests/rest_stream.rs              new  decoder boundaries
crates/cxas-core/tests/rest_stream_http.rs         new  streaming over a socket
crates/cxas-cli/src/commands/api.rs                new  list/describe/call/stream
crates/cxas-cli/src/args.rs                        edit api command family
crates/cxas-cli/tests/api.rs                       new  CLI end to end
crates/cxas-parity/tests/discovery_contract.rs     edit both-direction table check
docs/ci/reference-drift.yml                        edit table freshness in CI
```

---

# Phase 1 — The method table

## Task 1: Generate every method from the reference

- [x] Write `tools/generate_methods.py`: walk the nested `resources` tree of
      both vendored documents, sort, and emit a Rust file.
- [x] Emit to `crates/cxas-core/src/rest/method_table.rs`, checked in.
- [x] Add `--check` that reports staleness without writing.
- [x] Refuse to emit an empty table — a document that parsed to nothing would
      otherwise produce a table that satisfies every assertion vacuously.

```python
def collect(resources: dict | None, out: list) -> None:
    for _, resource in sorted((resources or {}).items()):
        for _, method in sorted((resource.get("methods") or {}).items()):
            out.append((method["id"], method["httpMethod"], method["path"]))
        collect(resource.get("resources"), out)
```

- [x] Tests in `tests/test_generate_methods.py`: nested descent, empty-resource
      tolerance, per-surface counts (66 / 104), verb and path shape, id
      uniqueness, deterministic output, `--check` passing on a fresh table and
      failing on a drifted or absent one.

## Task 2: Split types from data

- [x] Keep `MethodSpec`, `ApiVersion`, and lookups in `method.rs`; move the
      table to `method_table.rs`.
- [x] Add `MethodSpec::required_params` so a caller can name a missing template
      variable before expansion fails.
- [x] Add `MethodSpec::is_streaming` — the one method whose response must not be
      buffered.
- [x] Add `MODELLED`: the methods this workspace wraps in its own types and CLI
      verbs, kept separate so the coverage report cannot claim credit for
      generated breadth.

## Task 3: Check the table both ways

- [x] Add `declared_table_matches_discovery_exactly` asserting the table and the
      reference agree in both directions.

```rust
let missing: Vec<&&str> = upstream.difference(&declared).collect();
let invented: Vec<&&str> = declared.difference(&upstream).collect();
assert!(missing.is_empty() && invented.is_empty(), ...);
```

- [x] Add `modelled_methods_are_addressable` — nothing in `MODELLED` may name a
      method CES no longer declares, and no duplicate may inflate the count.
- [x] Add `every_method_declares_the_parameters_its_path_needs`.
- [x] Report addressable and modelled coverage separately; keep the report
      non-gating, because a threshold can be satisfied by deleting the metric.

---

# Phase 2 — Credentials

## Task 4: Resolve without I/O

- [x] Add `crates/cxas-core/src/auth.rs` with a `Host` trait over the
      environment, so precedence tests never mutate process state.
- [x] Implement `resolve(&dyn Host, Option<&str>) -> Result<CredentialSource, _>`
      in Google's own precedence order.
- [x] Parse credential files by declared `type`, reporting the *kind* even when
      unsupported.
- [x] Refuse service-account keys and external accounts by name, listing the
      supported alternatives.

```rust
AdcKind::ServiceAccount { client_email } => Err(CoreError::Auth(format!(
    "{origin} holds a service-account key for {client_email}; signing a key file is \
     not implemented. Use `gcloud auth application-default login`, ..."
))),
```

- [x] Add `CoreError::Auth`, separate from `Transport`: a transport failure is
      worth retrying, an auth failure is worth reading.
- [x] 24 tests: every precedence step, blank explicit token, unreadable path,
      no-fallthrough on an unsupported credential, per-platform well-known path,
      label never leaking the secret.

## Task 5: Mint, cache, refresh

- [x] Add `TokenProvider` behind `rest`: refresh-token grant, metadata server,
      `gcloud` subprocess, static passthrough.
- [x] Cache behind a `Mutex` so concurrent requests share one token.
- [x] Refresh `REFRESH_SKEW` before expiry so a token cannot die in flight.
- [x] Try `gcloud.cmd` before `gcloud` on Windows, where the bare name is not
      executable.
- [x] Write `crates/cxas-core/tests/support/mod.rs`: a scripted loopback server
      that captures request line, headers, and body.
- [x] 7 tests: grant shape, `Metadata-Flavor`, cache reuse, refresh inside the
      skew, revoked-grant reason preserved, static needing no socket, and
      exactly one authorization header reaching CES.

---

# Phase 3 — Streaming

## Task 6: Decode incrementally

- [x] Add `JsonStreamDecoder`: byte-oriented, tracking nesting, string, and
      escape state.

```rust
pub fn push(&mut self, chunk: &[u8]) -> Vec<String>;
pub fn finish(&mut self) -> Result<Option<String>, CoreError>;
```

- [x] Return only whole values, so each result is safe to hand to a JSON parser.
- [x] Treat a stream ending mid-message as an error — a dropped connection and a
      finished conversation are otherwise indistinguishable.
- [x] 17 tests: split messages, braces inside strings, escaped backslashes,
      nesting, multi-byte characters split across chunks, byte-at-a-time
      delivery, NDJSON, empty arrays, truncation after partial success.

## Task 7: Wire it to the socket

- [x] Add `CesHttpClient::stream`, feeding response chunks to the decoder and
      invoking a callback per message.
- [x] Prove incremental delivery: the stub withholds message two until the
      client's callback reports message one, with a bounded wait so the failure
      is a failure rather than a hang.

```rust
if let Some(rx) = &ack {
    if rx.recv_timeout(ACK_TIMEOUT).is_err() {
        *flag.lock().expect("flag") = false;
    }
}
```

- [x] 5 tests: incremental delivery, truncation keeping what arrived, 403 before
      any message, correct `:streamRunSession` URL, unary methods not marked
      streaming.

---

# Phase 4 — The CLI surface

## Task 8: `cxas api`

- [x] Add `cxas api list | describe | call | stream` behind a `remote` feature,
      on by default.
- [x] `list` and `describe` are offline, reading the same table `call`
      dispatches through.
- [x] Name a missing path parameter before opening a socket.
- [x] On an unknown id, report the other surface when the id exists there;
      otherwise suggest by shared prefix rather than by trailing word.

```rust
if let Some(elsewhere) = METHODS.iter().find(|m| m.id == id) {
    // "CES declares {id} on v1beta only, not on v1"
}
```

- [x] Reject a non-JSON `--body` locally, with the parse error.
- [x] Refuse `api stream` on a unary method rather than hanging.
- [x] 15 tests, offline and against a loopback stub.

## Task 9: Keep the docs true

- [x] Update README, `docs-site/{index,cli,architecture,crates,benchmark,limits}.html`,
      and the coverage map: 170 addressable, 37 modelled, both reported.
- [x] State in three places that the table check is a staleness check.
- [x] Add the table-freshness check and the Python suite to
      `docs/ci/reference-drift.yml`.
- [x] Clear the pre-existing clippy warnings so the signal is usable, with an
      `#[allow]` and a reason where the lint is wrong about a deliberate API
      name.

## Task 10: Verify

- [x] `cargo test --workspace` — 221 passing.
- [x] `cargo test -p cxas-core` (no `rest`) — 74 passing, proving the pure half
      compiles alone.
- [x] `cargo build -p cxas-cli --no-default-features` — clean.
- [x] `cargo clippy --workspace --all-targets` — clean.
- [x] `python -m pytest tests gauntlet/tests` — 45 passing.
- [x] `python tools/generate_methods.py --check` and
      `python tools/refresh_reference.py --check`.
- [x] `python tools/superpowers_validate.py`.

---

# Phase 5 — Deliberately not in this phase

Carried forward rather than quietly dropped. Each is unchecked because it is
genuinely not done, and each would change the claims on
`docs-site/limits.html` when it is.

- [ ] Move the catalog verbs (`create`, `push`, `apps list`, `deployments`)
      onto `CesHttpClient`, behind an explicit flag so the offline workflow
      survives. Today they read `.cxas/catalog.json` and `cxas api` is the only
      live path.
- [ ] Typed request and response bodies for the resources that matter. 133 of
      170 methods hand back raw JSON, which is honest but not ergonomic.
- [ ] Long-running operation polling. An `Operation` is returned as-is; there is
      no wait-for-completion helper.
- [ ] Retry and backoff for 429 and 503. Returned as typed errors today, with
      the caller deciding — which is defensible, but not what a client library
      should make every caller reimplement.
- [ ] Service-account key signing (RS256 JWT) and workload-identity federation
      (STS exchange). Both refuse by name today rather than falling through.
- [ ] Run the Gauntlet Loop against a live model. Check `max_agent_calls` in
      `gauntlet/config.toml` first: nine crates times eight rounds times two
      calls per round is 144 invocations, and the cap ships at 40.
- [ ] Install `docs/ci/reference-drift.yml` as a GitHub Action. Needs a token
      with the `workflow` scope; the checks run locally in the meantime.
