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
fn unknown_wire_value_is_typed_not_a_panic() {
    let state = EvaluationRunState::from_wire(99);
    assert_eq!(state, EvaluationRunState::Unknown(99));
    assert_eq!(state.as_str_name(), "UNKNOWN(99)");
}

#[test]
fn known_wire_value_maps() {
    assert_eq!(
        EvaluationRunState::from_wire(3),
        EvaluationRunState::Succeeded
    );
    assert_eq!(EvaluationRunState::Succeeded.as_str_name(), "SUCCEEDED");
}

#[test]
fn source_never_calls_name_on_i32() {
    let src = include_str!("../src/evaluation_run_state.rs");
    assert!(
        !src.contains(".name()"),
        "reintroduces the Python #284 crash class"
    );
}
