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
    assert_eq!(
        d.methods().count(),
        66,
        "v1 method count changed; re-pin the reference rather than editing this number"
    );
}

#[test]
fn vendored_v1beta_parses_with_expected_surface() {
    let d = Discovery::load(&reference("v1beta.discovery.json")).expect("v1beta must parse");
    assert_eq!(
        d.methods().count(),
        104,
        "v1beta method count changed; re-pin the reference rather than editing this number"
    );
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

#[test]
fn evaluation_resources_are_v1beta_only() {
    // The eval bugs (#284, #263, #355, #345, #136) all live on resources that
    // v1 does not expose at all. Anything modelling evals against v1 is wrong.
    let v1 = Discovery::load(&reference("v1.discovery.json")).expect("v1 must parse");
    let v1beta = Discovery::load(&reference("v1beta.discovery.json")).expect("v1beta must parse");

    let has_eval = |d: &Discovery| d.methods().any(|m| m.id.contains("evaluationRuns"));
    assert!(!has_eval(&v1), "v1 unexpectedly exposes evaluationRuns");
    assert!(has_eval(&v1beta), "v1beta must expose evaluationRuns");
}
