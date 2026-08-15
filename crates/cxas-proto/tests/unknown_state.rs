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
fn python_era_spellings_are_not_silently_accepted() {
    // PENDING/SUCCEEDED/FAILED were invented by this crate and never existed in
    // CES. They must land in Unknown, not resolve to a known variant.
    for invented in ["PENDING", "SUCCEEDED", "FAILED"] {
        assert_eq!(
            EvaluationRunState::from_wire_name(invented),
            EvaluationRunState::Unknown(invented.to_string()),
            "{invented} must not resolve to a known variant"
        );
    }
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
fn unknown_state_is_never_terminal() {
    // A poller must not exit on a state it has never seen: an unrecognized
    // value is not evidence the run finished.
    assert!(!EvaluationRunState::Unknown("FUTURE".to_string()).is_terminal());
}

#[test]
fn source_never_calls_name_on_i32() {
    let src = include_str!("../src/evaluation_run_state.rs");
    assert!(
        !src.contains(".name()"),
        "reintroduces the Python #284 crash class"
    );
}
