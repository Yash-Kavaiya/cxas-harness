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

use crate::AudioScore;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportSummary {
    pub passed: usize,
    pub failed: usize,
    pub errored: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectationResult {
    pub expected: String,
    pub actual: String,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnRow {
    pub case_id: String,
    pub turn_index: usize,
    pub user: String,
    pub agent_text: String,
    pub audio: Option<AudioScore>,
    pub expectation_results: Vec<ExpectationResult>,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalReport {
    pub summary: ReportSummary,
    pub turns: Vec<TurnRow>,
}

impl EvalReport {
    pub fn empty() -> Self {
        Self {
            summary: ReportSummary::default(),
            turns: Vec::new(),
        }
    }
}
