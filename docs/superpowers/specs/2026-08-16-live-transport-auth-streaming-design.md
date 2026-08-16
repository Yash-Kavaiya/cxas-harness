# Live transport, credentials, and streaming — design

**Date:** 2026-08-16
**Status:** implemented
**Supersedes:** the "Transport (phase 3)" section of
`docs/superpowers/specs/2026-08-15-discovery-benchmark-gauntlet-design.md`

## Purpose

`cxas-harness` could describe CES accurately and could not talk to it usefully.
The previous phase left three gaps, each of which made the harness a linter
rather than a client:

1. **35 of 170 methods.** The method table was hand-maintained, so it covered
   whatever somebody had needed so far and fell behind on every CES release.
2. **Bearer tokens only.** A caller had to mint a token elsewhere and paste it
   in. There was no Application Default Credentials discovery, and a token that
   expired mid-run failed in a way that read as a permissions problem.
3. **No streaming.** `streamRunSession` was unimplemented, so the one CES method
   whose value is incremental delivery could not be called at all.

This phase closes all three, and adds the CLI surface that makes them reachable
without writing Rust.

## Motivating problem

The honest version of the previous phase's claim was "35 of 170 methods, and the
other 135 are unimplemented". That number was truthful but structurally
unstable: it could only be improved by hand, one method at a time, and every
addition was an opportunity to mistype a path template that no type in the
workspace would catch.

The discovery documents already contain every path template, verb, and id. A
hand-maintained subset of a machine-readable document is a copy that can only
diverge. This phase deletes the copy.

The second-order risk is that generating the table makes the parity test
tautological — a table generated from X, checked against X. That is stated
outright rather than papered over. What the check still catches is *staleness*:
`tools/refresh_reference.py` can pull a newer revision without anyone
regenerating, at which point the table describes an API that is no longer the
pinned one. That is the failure that actually happens.

To keep the coverage claim meaningful, coverage is reported as two numbers.
`METHODS` is generated and says a request can be built and sent. `MODELLED` is
hand-written and says the workspace has a type for the resource and an opinion
about what its failures mean. The gap between them is the part that takes
judgement, so the gap is what gets published.

## Global constraints

- The reference remains the only source of truth for what CES is. No method,
  verb, or path is written by hand.
- Everything that can be pure, is. Request construction, credential
  *resolution*, and stream decoding do no I/O and are always compiled.
- The `rest` feature gates the HTTP stack only. A lint-only build pulls in no
  TLS, no reqwest, no tokio runtime.
- No test may require a cloud project, a credential, or a network. Loopback
  stubs only.
- An unusable credential is an error at the precedence where it was found,
  never a reason to try the next source.
- `cxas-harness` keeps the `cxas-scrapi` CLI shape; `cxas api` is additive and
  claims no Python counterpart.

## Architecture

```text
                       reference/ces/*.discovery.json   (pinned + sha256)
                                     |
                     tools/generate_methods.py          (offline, checked in)
                                     |
                       cxas-core::rest::method_table    METHODS: 170 specs
                                     |
   +-------------------+-------------+-------------+
   |                   |                           |
 method.rs          request.rs                  stream.rs
 MethodSpec         RestRequest                 JsonStreamDecoder
 MODELLED           status_to_error             (pure, byte-oriented)
   |                   |                           |
   +---------+---------+---------------------------+
             |
        rest/http.rs   CesHttpClient          [feature = "rest"]
             |            call() / stream()
             |
        auth.rs        TokenProvider          [feature = "rest"]
             |            resolve() is pure
             |
        cxas-cli::commands::api                [feature = "remote"]
             api list / describe / call / stream
```

The dependency direction never reverses: the CLI knows about `cxas-core`, and
`cxas-core` knows about the reference. Nothing reads a discovery document at
runtime.

## Data flow

A `cxas api call` from argv to exit code:

1. **Parse.** clap yields the method id, `--param`/`--query` pairs, `--body`.
2. **Resolve the method.** `resolve_method(id)` looks the id up in `METHODS`,
   preferring `v1` where both surfaces declare it. A miss that exists on the
   other surface reports *that*, because every evaluation method is
   `v1beta`-only and naming one on `v1` is the most common mistake.
3. **Validate locally.** `MethodSpec::required_params` names any template
   variable the caller omitted; `--body` is parsed as JSON. Both fail before a
   socket is opened, so the answer is the parameter name rather than a 404.
4. **Resolve a credential.** `auth::resolve` inspects an environment snapshot
   and returns a `CredentialSource`. No I/O.
5. **Mint a token.** `TokenProvider::token` returns the cached token if it is
   valid past the refresh skew, otherwise performs the one round trip its
   source requires — a refresh-token grant, a metadata GET, or a `gcloud`
   subprocess.
6. **Build.** `RequestBuilder::build` expands the path template (RFC 6570
   reserved expansion for `{+var}`), sorts and encodes the query, and assembles
   headers. Pure and fully assertable.
7. **Send.** `CesHttpClient` issues it. Non-2xx becomes a typed `CoreError`
   carrying the status and the service's own message.
8. **Stream, if streaming.** Response bytes are fed to `JsonStreamDecoder`,
   which emits whole JSON values as chunk boundaries allow. Each is handed to
   the caller immediately.
9. **Report.** A JSON envelope on stdout; exit 0, 1, or 2.

### Credential precedence

Highest first. This mirrors Google's own client libraries, so a machine already
configured for `gcloud` behaves identically here.

| Order | Source | I/O to mint |
|---|---|---|
| 1 | `--oauth-token` | none |
| 2 | `CXAS_ACCESS_TOKEN` | none |
| 3 | `GOOGLE_APPLICATION_CREDENTIALS` | refresh-token grant |
| 4 | well-known ADC file | refresh-token grant |
| 5 | metadata server | one GET |
| 6 | `gcloud auth print-access-token` | one subprocess |

Sources 3 and 4 accept authorized-user files only. A service-account key or an
external-account credential is reported by name, with the supported
alternatives listed.

### Stream framing

CES answers `streamRunSession` with a JSON array delivered in chunks whose
boundaries have nothing to do with message boundaries. The decoder tracks
nesting depth, string state, and escape state over *bytes*, because a chunk
boundary can land inside a multi-byte character and decoding each chunk as UTF-8
on arrival would corrupt it. A complete JSON value always ends on an ASCII
delimiter, so the conversion is safe once a whole value is in hand.

## Components

### `tools/generate_methods.py`

Walks the nested `resources` tree of both vendored documents and emits
`crates/cxas-core/src/rest/method_table.rs`. Sorted output, so regeneration
produces a byte-identical file and a real change shows as a real diff.
`--check` reports staleness without writing.

Checked in rather than built by `build.rs`: a build script that parsed JSON
would make every downstream build depend on the reference files, and one that
fetched them would make builds depend on the network.

### `cxas-core::rest::method`

`MethodSpec`, `ApiVersion`, `method_spec`, `resolve_method`, and `MODELLED`.
`MethodSpec::required_params` parses the path template so callers can report a
missing parameter before expansion fails. `MethodSpec::is_streaming` identifies
the one method whose response must not be buffered.

### `cxas-core::rest::stream`

`JsonStreamDecoder`. `push(&[u8]) -> Vec<String>` returns only whole values;
`finish() -> Result<Option<String>, CoreError>` reports a stream that ended
mid-message. Pure, so every boundary case is a plain function call rather than
something to hope a live service reproduces.

### `cxas-core::auth`

`Host` (a trait over the environment), `resolve`, `CredentialSource`,
`CachedToken`, `parse_credential_file`, `parse_token_response`, and — behind
`rest` — `TokenProvider`. The split exists so precedence is testable without
mutating process environment, which cannot be done safely in parallel and would
otherwise make the tests depend on whether the developer is logged into gcloud.

`TokenProvider` caches behind a `Mutex` rather than taking `&mut self`, so one
provider shared by concurrent requests mints one token rather than ten.

### `cxas-core::rest::http`

`CesHttpClient` gains `discover`, `with_tokens`, and `stream`. Provider-minted
tokens *replace* any builder-carried authorization header rather than being
appended; two authorization headers is a request CES rejects rather than
adjudicates.

### `cxas-cli::commands::api`

`list`, `describe`, `call`, `stream`. Generic rather than one subcommand per
method: 170 hand-written verbs would add a spelling to remember and nothing
else. What a caller cannot do unaided is discover ids, know which surface
declares them, and see which parameters a path needs — so `list` and `describe`
do that, offline, from the same table `call` dispatches through.

## Error handling

| Condition | Result |
|---|---|
| Unknown method id | `UNKNOWN_METHOD`, exit 2. If the id exists on the other surface, say so; otherwise suggest by shared prefix. |
| Missing template parameter | `MISSING_PARAMETER` naming it, exit 2, before any I/O |
| `--body` not JSON | `USAGE`, exit 2, with the parse error |
| `api stream` on a unary method | `NOT_STREAMING`, exit 2 |
| No usable credential | `CoreError::Auth`, exit 1, naming what was found and what is supported |
| Service-account key | `CoreError::Auth` naming the account — never a fallback |
| Revoked refresh token | the endpoint's own `error_description` preserved |
| Non-2xx from CES | `CoreError::Transport` with status and the service's message |
| Stream ends mid-message | `CoreError::Transport`; messages already delivered are still reported |

`CoreError::Auth` is separate from `CoreError::Transport` because the remedy
differs: a transport failure is worth retrying, an auth failure is worth
reading.

## Testing

No test touches the network or a credential.

| Area | Kind | Count |
|---|---|---|
| Credential precedence | pure, fake `Host` | 24 |
| Token minting, caching, refresh | loopback stub | 7 |
| Stream decoding | pure, byte-level | 17 |
| Streaming over HTTP | loopback stub | 5 |
| `cxas api` | in-process + loopback stub | 15 |
| Method table generation | Python | 11 |

Three properties are asserted that a live-service test could not reproduce on
demand:

- **Incremental delivery.** The stub refuses to send message two until message
  one has been reported by the client's callback, so a client that buffered the
  whole body would stall. A bounded wait turns that into a failure rather than
  a hang.
- **Truncation is not a short read.** A stream cut mid-message is an error, and
  the messages that arrived whole are still delivered.
- **One authorization header.** The builder carries a stale token and the
  provider mints a fresh one; exactly one header must reach the wire, carrying
  the fresh value.

## Consequences accepted

- The parity test over the method table is a staleness check, not independent
  verification. Documented as such in three places rather than implied.
- 133 methods have no typed request or response body. Callers get raw JSON.
- No retry, no backoff, no long-running-operation polling. A 429 is returned as
  a typed error and the caller decides.
- Service-account key signing and workload-identity federation are unimplemented.
- The catalog commands still read `.cxas/catalog.json`. `cxas api` is the live
  path; moving the catalog verbs onto it is separate work.
- Nothing here has been verified against a live CES project. The claim is that
  requests match Google's machine-readable description of the API — a strong
  claim, and a different one from "this works in production".

## Issue-driven quality bar

This phase adds no new closer for the 25 cataloged `cxas-scrapi` issues; it
strengthens two that already exist. #263 (evaluation runs contending with the
session quota) now has a real 429 path through `status_to_error` rather than a
fixture. #284 (`EvaluationRunState` drift) keeps its discovery-backed assertion
and gains the surrounding guarantee that the method table it lives beside cannot
silently fall behind the same reference.

The bar itself is unchanged: every enum and method the Rust crates declare must
resolve against `reference/ces/`, and every cataloged issue must have a closing
test exercising behaviour verified against discovery rather than against a test
double asserting the code's own assumptions.
