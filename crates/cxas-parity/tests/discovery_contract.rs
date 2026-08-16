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

use cxas_core::{ApiVersion, METHODS};
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
fn declared_methods_resolve_in_discovery() {
    // Every method cxas-core claims to implement must exist in the vendored
    // discovery document with the same verb and the same path template. A typo
    // in a path is otherwise invisible until a live request 404s.
    let v1 = load("v1");
    let v1beta = load("v1beta");
    let mut failures = Vec::new();

    for spec in METHODS {
        let (doc, label) = match spec.api_version {
            ApiVersion::V1 => (&v1, "v1"),
            ApiVersion::V1Beta => (&v1beta, "v1beta"),
        };

        let Some(actual) = doc.method(spec.id) else {
            failures.push(format!("{} is not declared in {label} discovery", spec.id));
            continue;
        };
        if actual.http_method != spec.http_method {
            failures.push(format!(
                "{}: declared verb {} != discovery {}",
                spec.id, spec.http_method, actual.http_method
            ));
        }
        if actual.path != spec.path {
            failures.push(format!(
                "{}: declared path {} != discovery {}",
                spec.id, spec.path, actual.path
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "declared methods drifted from vendored CES discovery:\n  {}",
        failures.join("\n  ")
    );
}

#[test]
fn evaluation_methods_are_never_declared_against_v1() {
    // v1 exposes no evaluation resources at all. Declaring one there would
    // build a URL that can only ever 404.
    for spec in METHODS {
        if spec.id.contains("evaluation") || spec.id.contains("Evaluation") {
            assert_eq!(
                spec.api_version,
                ApiVersion::V1Beta,
                "{} is an evaluation method declared against v1",
                spec.id
            );
        }
    }
}

#[test]
fn coverage_report_counts_implemented_methods() {
    // Reports, never gates. A pass/fail threshold can be satisfied by deleting
    // the metric; a printed number cannot.
    let v1 = load("v1");
    let v1beta = load("v1beta");
    let (n1, nbeta) = (v1.methods().count(), v1beta.methods().count());

    let impl_v1 = METHODS
        .iter()
        .filter(|m| m.api_version == ApiVersion::V1)
        .count();
    let impl_beta = METHODS
        .iter()
        .filter(|m| m.api_version == ApiVersion::V1Beta)
        .count();

    println!(
        "CES-COVERAGE v1={impl_v1}/{n1} v1beta={impl_beta}/{nbeta} total={}/{}          v1_revision={} v1beta_revision={}",
        impl_v1 + impl_beta,
        n1 + nbeta,
        v1.revision(),
        v1beta.revision()
    );
    assert!(n1 + nbeta > 0, "discovery documents must declare methods");
}
