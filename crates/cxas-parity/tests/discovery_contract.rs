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

use cxas_core::{ApiVersion, METHODS, MODELLED};
use cxas_discovery::Discovery;
use cxas_proto::enum_registry::REGISTERED_ENUMS;
use std::collections::BTreeSet;
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
fn declared_table_matches_discovery_exactly() {
    // The table is generated from these documents, so this is a staleness
    // check, not an independent verification -- and staleness is the failure
    // that actually happens. `tools/refresh_reference.py` can pull a newer
    // revision without anyone re-running `tools/generate_methods.py`, at which
    // point the table describes an API that is no longer the pinned one.
    //
    // Checked in both directions on purpose. Only checking that declared
    // methods exist upstream would pass a table that dropped half of CES.
    for (label, version) in [("v1", ApiVersion::V1), ("v1beta", ApiVersion::V1Beta)] {
        let doc = load(label);
        let declared: BTreeSet<&str> = METHODS
            .iter()
            .filter(|m| m.api_version == version)
            .map(|m| m.id)
            .collect();
        let upstream: BTreeSet<&str> = doc.methods().map(|m| m.id.as_str()).collect();

        let missing: Vec<&&str> = upstream.difference(&declared).collect();
        let invented: Vec<&&str> = declared.difference(&upstream).collect();

        assert!(
            missing.is_empty() && invented.is_empty(),
            "{label} method table is stale -- re-run tools/generate_methods.py
               missing from the table: {missing:?}
               absent from discovery:  {invented:?}"
        );
    }
}

#[test]
fn modelled_methods_are_addressable() {
    // MODELLED is hand-maintained; the table is generated. A modelled id that
    // no longer resolves means CES renamed or withdrew something this
    // workspace still claims to model.
    let mut unresolved = Vec::new();
    for id in MODELLED {
        if cxas_core::resolve_method(id).is_none() {
            unresolved.push(*id);
        }
    }
    assert!(
        unresolved.is_empty(),
        "modelled methods that CES no longer declares: {unresolved:?}"
    );

    let unique: BTreeSet<&&str> = MODELLED.iter().collect();
    assert_eq!(
        unique.len(),
        MODELLED.len(),
        "MODELLED lists the same method twice, which would inflate the coverage report"
    );
}

#[test]
fn every_method_declares_the_parameters_its_path_needs() {
    // A path template naming a variable the caller cannot know about is a
    // request that can only fail at expansion time.
    for spec in METHODS {
        let params = spec.required_params();
        assert!(
            !params.is_empty(),
            "{} has no template parameters, which no CES method does",
            spec.id
        );
        for name in params {
            assert!(
                !name.is_empty(),
                "{} has an empty template parameter in {}",
                spec.id,
                spec.path
            );
        }
    }
}

#[test]
fn coverage_report_counts_addressable_and_modelled_separately() {
    // Reports, never gates. A pass/fail threshold can be satisfied by deleting
    // the metric; a printed number cannot.
    //
    // Two numbers, because they mean different things. Addressable coverage is
    // generated and therefore cheap -- it says a request can be built and sent.
    // Modelled coverage is hand-written and says this workspace has an opinion
    // about what the resource is and what failure means for it.
    let v1 = load("v1");
    let v1beta = load("v1beta");
    let (n1, nbeta) = (v1.methods().count(), v1beta.methods().count());

    let addr_v1 = METHODS
        .iter()
        .filter(|m| m.api_version == ApiVersion::V1)
        .count();
    let addr_beta = METHODS
        .iter()
        .filter(|m| m.api_version == ApiVersion::V1Beta)
        .count();

    println!(
        "CES-COVERAGE addressable v1={addr_v1}/{n1} v1beta={addr_beta}/{nbeta} total={}/{} modelled={}/{} v1_revision={} v1beta_revision={}",
        addr_v1 + addr_beta,
        n1 + nbeta,
        MODELLED.len(),
        n1 + nbeta,
        v1.revision(),
        v1beta.revision()
    );
    assert!(n1 + nbeta > 0, "discovery documents must declare methods");
}
