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

use cxas_evals::{generate_combined_json_report, EvalReport, TurnRow};

#[test]
fn json_report_includes_turn_rows() {
    let report = EvalReport {
        summary: cxas_evals::ReportSummary {
            passed: 1,
            failed: 0,
            errored: 0,
        },
        turns: vec![
            TurnRow {
                case_id: "c1".into(),
                turn_index: 0,
                user: "hi".into(),
                agent_text: "hello".into(),
                audio: None,
                expectation_results: vec![],
                latency_ms: 3,
            },
            TurnRow {
                case_id: "c1".into(),
                turn_index: 1,
                user: "bye".into(),
                agent_text: "goodbye".into(),
                audio: None,
                expectation_results: vec![],
                latency_ms: 4,
            },
        ],
    };
    let json = generate_combined_json_report(&report);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["turns"].as_array().unwrap().len(), 2);
    assert_eq!(v["turns"][1]["turn_index"], 1);
}

#[test]
fn parity_eval_types_exist() {
    assert_eq!(cxas_evals::ToolEvals::crate_label(), "ToolEvals");
    assert_eq!(cxas_evals::CallbackEvals::crate_label(), "CallbackEvals");
    assert_eq!(cxas_evals::GuardrailEvals::crate_label(), "GuardrailEvals");
    assert_eq!(cxas_evals::TurnEvals::crate_label(), "TurnEvals");
    assert_eq!(
        cxas_evals::SimulationEvals::crate_label(),
        "SimulationEvals"
    );
}
