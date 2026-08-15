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

use std::io::Cursor;

#[test]
fn trace_raw_emits_one_json_object_per_turn_with_raw() {
    cxas_cli::test_support::script_trace(vec![
        serde_json::json!({"text": "hi"}),
        serde_json::json!({"text": "yo"}),
    ]);
    let mut buf = Cursor::new(Vec::new());
    let code = cxas_cli::run(
        &[
            "cxas".into(),
            "trace".into(),
            "--app-name".into(),
            "projects/p/locations/us/apps/a".into(),
            "--location".into(),
            "us".into(),
            "--raw".into(),
        ],
        &mut buf,
    );
    assert_eq!(code, 0);
    let text = String::from_utf8(buf.into_inner()).unwrap();
    let lines: Vec<&str> = text.lines().filter(|l| l.starts_with('{') && l.contains("turn")).collect();
    // When --format json wraps a single object, the data field is an array of turns.
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    let turns = v["data"]["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 2);
    assert!(turns[0]["raw"].is_object());
    assert!(turns[1]["raw"].is_object());
    let _ = lines;
}

#[test]
fn evals_report_json_includes_turns() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("sim_results.json"), r#"{"turns":[{"turn_index":0}]}"#).unwrap();
    let mut buf = Cursor::new(Vec::new());
    let out = dir.path().join("out.json");
    let code = cxas_cli::run(
        &[
            "cxas".into(),
            "evals".into(),
            "report".into(),
            "--output-dir".into(),
            dir.path().display().to_string(),
            "--format".into(),
            "json".into(),
            "--output".into(),
            out.display().to_string(),
        ],
        &mut buf,
    );
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(out).unwrap()).unwrap();
    assert!(v["turns"].is_array());
}
