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
fn lint_json_is_parseable_and_non_interactive() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("app.yaml"), "display_name: d\n").unwrap();
    let mut buf = Cursor::new(Vec::new());
    let code = cxas_cli::run(
        &[
            "cxas".into(),
            "lint".into(),
            "--app-dir".into(),
            dir.path().display().to_string(),
        ],
        &mut buf,
    );
    let v: serde_json::Value = serde_json::from_slice(buf.get_ref()).unwrap();
    assert!(v["ok"].is_boolean());
    assert_eq!(v["command"], "lint");
    assert_eq!(code, 1, "missing root_agent is an error");
}

#[test]
fn pull_without_location_or_workspace_is_usage() {
    let mut buf = Cursor::new(Vec::new());
    let code = cxas_cli::run(
        &[
            "cxas".into(),
            "pull".into(),
            "--app".into(),
            "demo".into(),
            "--target-dir".into(),
            "/tmp/out".into(),
        ],
        &mut buf,
    );
    assert_eq!(code, 2);
    let v: serde_json::Value = serde_json::from_slice(buf.get_ref()).unwrap();
    assert_eq!(v["error"]["code"], "LOCATION_REQUIRED");
}
