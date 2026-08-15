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

use crate::{EvalReport, ExpectationResult, ReportSummary, TurnRow};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Default, Deserialize)]
struct FixtureDoc {
    #[serde(default)]
    cases: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
struct FixtureCase {
    #[serde(default)]
    id: String,
    expected: serde_json::Value,
    #[serde(default)]
    actual: serde_json::Value,
}

pub(crate) fn run_expectation_fixture(path: impl AsRef<Path>) -> EvalReport {
    let path = path.as_ref();
    let Ok(raw) = std::fs::read_to_string(path) else {
        return EvalReport::empty();
    };
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let doc: FixtureDoc = if matches!(ext, "yaml" | "yml") {
        serde_yaml::from_str(&raw).unwrap_or_default()
    } else {
        serde_json::from_str(&raw).unwrap_or_default()
    };

    let mut summary = ReportSummary::default();
    let mut turns = Vec::new();
    for (idx, case) in doc.cases.into_iter().enumerate() {
        let passed = case.expected == case.actual;
        if passed {
            summary.passed += 1;
        } else {
            summary.failed += 1;
        }
        let expected = stringify_value(&case.expected);
        let actual = stringify_value(&case.actual);
        turns.push(TurnRow {
            case_id: if case.id.is_empty() {
                format!("case-{idx}")
            } else {
                case.id
            },
            turn_index: idx,
            user: expected.clone(),
            agent_text: actual.clone(),
            audio: None,
            expectation_results: vec![ExpectationResult {
                expected,
                actual,
                passed,
            }],
            latency_ms: 0,
        });
    }
    EvalReport { summary, turns }
}

fn stringify_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}
