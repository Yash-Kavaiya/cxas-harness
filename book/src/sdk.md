# Core SDK

Rust crates that the `cxas` binary delegates to. Full API notes: <https://yash-kavaiya.github.io/cxas-harness/crates.html>

| Crate | Role |
|---|---|
| `cxas-parity` | Frozen `cxas-scrapi` public surface (YAML) |
| `cxas-proto` | `EvaluationRunState::Unknown(i32)` |
| `cxas-core` | `Location`, Apps export stream, `QuotaKind`, channels |
| `cxas-utils` | Pagination + boolean environment templates |
| `cxas-state` | Content-addressed hash / diff / cascading profiles |
| `cxas-evals` | `TurnCursor`, `BidiSession`, `AudioScorer`, reports |
| `cxas-lint` | Rule registry, `V-ROOT`, welcome / depver |
| `cxas-migration` | `SnapshotGuard`, `ToolSync`, DFCX pipeline |
| `cxas-cli` | `cxas` binary |

```rust
use cxas_core::Location;

let loc = Location::new("us").expect("caller provided a region");
assert_eq!(loc.as_str(), "us");
assert!(Location::new("").is_err());
assert!(Location::new("__default_global__").is_err());
```
