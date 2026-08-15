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

use std::fs;
use std::io::Cursor;

#[test]
fn actions_init_writes_matrix_for_each_environment() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("app.yaml"), "display_name: pilot\nroot_agent: main\n").unwrap();
    fs::write(dir.path().join("environment.json"), r#"{"dev":{},"prod":{}}"#).unwrap();
    let mut buf = Cursor::new(Vec::new());
    let code = cxas_cli::run(
        &[
            "cxas".into(),
            "actions".into(),
            "init".into(),
            "--app-dir".into(),
            dir.path().display().to_string(),
        ],
        &mut buf,
    );
    assert_eq!(code, 0);
    let wf = fs::read_to_string(dir.path().join(".github/workflows/test_pilot.yml")).unwrap();
    assert!(wf.contains("dev"));
    assert!(wf.contains("prod"));
    assert!(wf.contains("cxas lint"));
}

#[test]
fn init_github_action_alias_writes_the_same_workflow() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("app.yaml"), "display_name: pilot\nroot_agent: main\n").unwrap();
    let mut buf = Cursor::new(Vec::new());
    let code = cxas_cli::run(
        &[
            "cxas".into(),
            "init-github-action".into(),
            "--app-dir".into(),
            dir.path().display().to_string(),
        ],
        &mut buf,
    );
    assert_eq!(code, 0);
    assert!(dir.path().join(".github/workflows/test_pilot.yml").exists());
}

#[test]
fn auto_create_wif_is_manual() {
    let mut buf = Cursor::new(Vec::new());
    let code = cxas_cli::run(
        &[
            "cxas".into(),
            "actions".into(),
            "init".into(),
            "--app-dir".into(),
            ".".into(),
            "--auto-create-wif".into(),
        ],
        &mut buf,
    );
    assert_eq!(code, 2);
    let v: serde_json::Value = serde_json::from_slice(buf.get_ref()).unwrap();
    assert_eq!(v["error"]["code"], "WIF_MANUAL");
}
