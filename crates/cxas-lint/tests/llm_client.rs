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

use cxas_lint::{Diagnostic, LintReport, Severity};
use std::path::PathBuf;

#[cfg(feature = "llm")]
use std::sync::Mutex;

#[cfg(feature = "llm")]
static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn json_report_contains_stable_fields() {
    let report = LintReport {
        diagnostics: vec![Diagnostic {
            rule_id: "V-ROOT".into(),
            severity: Severity::Error,
            path: PathBuf::from("app.yaml"),
            message: "missing root_agent".into(),
            fix: None,
        }],
    };
    let v: serde_json::Value = serde_json::from_str(&report.to_json()).unwrap();
    assert_eq!(v["diagnostics"][0]["rule_id"], "V-ROOT");
    assert_eq!(report.exit_code(), 1);
}

#[cfg(feature = "llm")]
#[tokio::test]
async fn llm_client_maps_json_array() {
    let _guard = ENV_LOCK.lock().unwrap();
    let (url, _join) = cxas_lint::test_support::spawn_json_listener(
        r#"[{"severity":"warning","message":"vague","path":"instruction.txt"}]"#,
    )
    .await;
    // SAFETY: serialized by ENV_LOCK; rustc 1.87+ requires unsafe for process env mutation.
    unsafe { std::env::set_var("CXAS_GEMINI_API_KEY", "test") };
    let client = cxas_lint::LlmLintClient::new(&url);
    let diags = client
        .lint_instructions(&[cxas_lint::InstructionFile {
            path: PathBuf::from("instruction.txt"),
            body: "be nice".into(),
        }])
        .await
        .unwrap();
    assert_eq!(diags[0].rule_id, "LLM-SEMANTIC");
    assert_eq!(diags[0].message, "vague");
}

#[cfg(feature = "llm")]
#[tokio::test]
async fn llm_client_rejects_non_json() {
    let _guard = ENV_LOCK.lock().unwrap();
    let (url, _join) = cxas_lint::test_support::spawn_json_listener("not json").await;
    unsafe { std::env::set_var("CXAS_GEMINI_API_KEY", "test") };
    let err = cxas_lint::LlmLintClient::new(&url)
        .lint_instructions(&[])
        .await
        .unwrap_err();
    assert!(matches!(err, cxas_lint::LintError::UnparseableModel));
}

#[cfg(feature = "llm")]
#[tokio::test]
async fn llm_client_requires_api_key() {
    let _guard = ENV_LOCK.lock().unwrap();
    unsafe { std::env::remove_var("CXAS_GEMINI_API_KEY") };
    let err = cxas_lint::LlmLintClient::new("http://127.0.0.1:1")
        .lint_instructions(&[])
        .await
        .unwrap_err();
    assert!(matches!(err, cxas_lint::LintError::MissingApiKey(_)));
}
