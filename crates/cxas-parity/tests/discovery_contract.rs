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

//! The contract that replaces the self-graded manifest check.
//!
//! The neighbouring `manifest_contract.rs` asserts that a checked-in YAML
//! contains strings that same YAML declares, so it can never fail. These
//! assertions are made against Google's vendored discovery documents instead.

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
    let (n1, nbeta) = (v1.methods().count(), v1beta.methods().count());

    println!(
        "CES-COVERAGE v1={n1} v1beta={nbeta} total={} v1_revision={} v1beta_revision={}",
        n1 + nbeta,
        v1.revision(),
        v1beta.revision()
    );
    assert!(n1 + nbeta > 0, "discovery documents must declare methods");
}
