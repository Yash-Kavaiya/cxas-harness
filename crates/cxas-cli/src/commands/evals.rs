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

use crate::commands::opt_str;
use crate::output::{write_err, write_ok, OutputFormat};
use clap::ArgMatches;
use cxas_evals::{
    generate_combined_html_report, generate_combined_json_report, EvalReport, TurnRow,
};
use std::fs;
use std::io::Write;
use std::path::Path;

pub fn run(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let output_dir = opt_str(matches, "output-dir").unwrap_or_else(|| ".".into());
    let dest = opt_str(matches, "output");
    let report = load_report(&Path::new(&output_dir).join("sim_results.json"));
    let body = match format {
        OutputFormat::Json => generate_combined_json_report(&report),
        OutputFormat::Human => generate_combined_html_report(&report),
    };
    if let Some(path) = dest.as_deref() {
        if let Err(err) = fs::write(path, &body) {
            return write_err(out, format, "evals report", "IO", &err.to_string(), 1);
        }
    }
    let data = serde_json::from_str(&generate_combined_json_report(&report))
        .unwrap_or_else(|_| serde_json::json!({ "turns": report.turns }));
    write_ok(
        out,
        format,
        "evals report",
        data,
        &format!("wrote {} turn(s)", report.turns.len()),
    )
}

fn load_report(path: &Path) -> EvalReport {
    let Ok(text) = fs::read_to_string(path) else {
        return EvalReport::empty();
    };
    if let Ok(report) = serde_json::from_str::<EvalReport>(&text) {
        return report;
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return EvalReport::empty();
    };
    let mut report = EvalReport::empty();
    if let Some(turns) = value.get("turns").and_then(|t| t.as_array()) {
        for (i, turn) in turns.iter().enumerate() {
            report.turns.push(TurnRow {
                case_id: turn
                    .get("case_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                turn_index: turn
                    .get("turn_index")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(i as u64) as usize,
                user: turn
                    .get("user")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                agent_text: turn
                    .get("agent_text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                audio: None,
                expectation_results: Vec::new(),
                latency_ms: turn.get("latency_ms").and_then(|v| v.as_u64()).unwrap_or(0),
            });
        }
    }
    report
}
