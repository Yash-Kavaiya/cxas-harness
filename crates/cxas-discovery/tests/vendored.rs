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
    // Asserted as the exact ordered list, not with `contains`: a containment
    // check passes happily while extra invented variants sit alongside the
    // real ones, which is how PENDING/SUCCEEDED/FAILED survived 78 tests.
    let d = Discovery::load(&reference("v1beta.discovery.json")).expect("v1beta must parse");
    let e = d
        .enum_field("EvaluationRun", "state")
        .expect("EvaluationRun.state must exist in v1beta");
    assert_eq!(
        e.values,
        vec![
            "EVALUATION_RUN_STATE_UNSPECIFIED",
            "QUEUED",
            "RUNNING",
            "COMPLETED",
            "ERROR",
            "CANCELLED",
        ]
    );
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

#[test]
fn array_valued_enum_properties_are_visible() {
    // `Conversation.inputTypes` is an array whose *items* carry the enum. A
    // parser that only reads `properties.<p>.enum` cannot see it, so nothing
    // downstream can be parity-checked against it -- the same blindness that
    // let EvaluationRunState declare PENDING/SUCCEEDED/FAILED unchallenged.
    let d = Discovery::load(&reference("v1beta.discovery.json")).expect("v1beta must parse");
    let e = d
        .enum_field("Conversation", "inputTypes")
        .expect("Conversation.inputTypes must be visible as an enum field");
    assert_eq!(
        e.values,
        vec![
            "INPUT_TYPE_UNSPECIFIED",
            "INPUT_TYPE_TEXT",
            "INPUT_TYPE_EVENT",
            "INPUT_TYPE_AUDIO",
            "INPUT_TYPE_IMAGE",
            "INPUT_TYPE_BLOB",
            "INPUT_TYPE_TOOL_RESPONSE",
            "INPUT_TYPE_VARIABLES",
        ]
    );
}

#[test]
fn repeated_enum_property_is_distinguishable_from_scalar() {
    // Reading items-level enums is not enough: a consumer that cannot tell
    // `Vec<InputType>` from `InputType` will model the wire format wrongly
    // while every value-level assertion still passes.
    let d = Discovery::load(&reference("v1beta.discovery.json")).expect("v1beta must parse");
    assert!(
        d.enum_field("Conversation", "inputTypes")
            .expect("inputTypes")
            .repeated,
        "inputTypes is declared `type: array` in discovery"
    );
    assert!(
        !d.enum_field("EvaluationRun", "state")
            .expect("state")
            .repeated,
        "state is a scalar string in discovery"
    );
}

#[test]
fn method_parameter_enums_are_visible() {
    // CES declares enums in three places; query parameters are the third. The
    // conversation `view` and `source` filters are wire enums with no schema
    // property behind them, so a schema-only parser cannot check them at all.
    let d = Discovery::load(&reference("v1beta.discovery.json")).expect("v1beta must parse");
    let e = d
        .parameter_enum("ces.projects.locations.apps.conversations.get", "view")
        .expect("conversations.get must declare a `view` enum parameter");
    assert_eq!(
        e.values,
        vec![
            "CONVERSATION_VIEW_UNSPECIFIED",
            "CONVERSATION_VIEW_BASIC",
            "CONVERSATION_VIEW_FULL",
        ]
    );
    assert!(
        d.parameter_enum("ces.projects.locations.apps.conversations.get", "name")
            .is_none(),
        "`name` is a path parameter with no enum"
    );
}

#[test]
fn repeated_enum_parameter_is_distinguishable_from_scalar() {
    // `source` is being deprecated in favour of the repeated `sources`; both
    // carry the same values, so only the arity tells them apart.
    let d = Discovery::load(&reference("v1beta.discovery.json")).expect("v1beta must parse");
    let list = "ces.projects.locations.apps.conversations.list";
    assert!(d.parameter_enum(list, "sources").expect("sources").repeated);
    assert!(!d.parameter_enum(list, "source").expect("source").repeated);
}

#[test]
fn every_enum_declaration_in_v1_is_visible() {
    // A census, not a sample. If a future parser change re-hides a whole class
    // of enum declaration, the specific tests above still pass and only this
    // count moves. Re-pin the reference rather than editing these numbers.
    let d = Discovery::load(&reference("v1.discovery.json")).expect("v1 must parse");
    assert_eq!(d.enum_fields().count(), 48, "v1 schema enum count changed");
    assert_eq!(
        d.parameter_enums().count(),
        5,
        "v1 parameter enum count changed"
    );
}

#[test]
fn every_enum_declaration_in_v1beta_is_visible() {
    let d = Discovery::load(&reference("v1beta.discovery.json")).expect("v1beta must parse");
    assert_eq!(
        d.enum_fields().count(),
        86,
        "v1beta schema enum count changed"
    );
    assert_eq!(
        d.parameter_enums().count(),
        5,
        "v1beta parameter enum count changed"
    );
}
