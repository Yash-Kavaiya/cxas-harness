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

pub fn generate_combined_json_report(report: &EvalReport) -> String {
    serde_json::to_string_pretty(report).unwrap_or_else(|_| "{\"turns\":[]}".into())
}

pub fn generate_combined_html_report(report: &EvalReport) -> String {
    let mut rows = String::new();
    for turn in &report.turns {
        let audio = match &turn.audio {
            Some(score) => format!(
                "{} ({:.2})",
                html_escape(&score.transcript),
                score.match_score
            ),
            None => String::new(),
        };
        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            html_escape(&turn.case_id),
            turn.turn_index,
            html_escape(&turn.user),
            html_escape(&turn.agent_text),
            audio,
            turn.latency_ms
        ));
    }
    format!(
        "<!DOCTYPE html><html><body><h1>Eval report</h1>\
         <p>passed={} failed={} errored={}</p>\
         <table><thead><tr><th>case</th><th>turn</th><th>user</th><th>agent</th><th>audio</th><th>latency_ms</th></tr></thead>\
         <tbody>{}</tbody></table></body></html>",
        report.summary.passed, report.summary.failed, report.summary.errored, rows
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
