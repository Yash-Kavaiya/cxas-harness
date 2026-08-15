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

use cxas_lint::{discover, RuleRegistry};
use std::fs;

#[test]
fn web_widget_without_welcome_event_fails() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("app.yaml"), "display_name: d\nroot_agent: main\n").unwrap();
    fs::create_dir_all(tmp.path().join("agents/main")).unwrap();
    fs::write(tmp.path().join("agents/main/instruction.txt"), "x").unwrap();
    fs::create_dir_all(tmp.path().join("deployments/web")).unwrap();
    fs::write(
        tmp.path().join("deployments/web/deployment.yaml"),
        "channel_type: WEB_WIDGET\napp_version: v1\n",
    )
    .unwrap();
    let report = RuleRegistry::builtin().run_all(&discover(tmp.path()).unwrap());
    assert!(report.diagnostics.iter().any(|d| d.rule_id == "V-WELCOME"));
}

#[test]
fn empty_app_version_fails_depver() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("app.yaml"), "display_name: d\nroot_agent: main\n").unwrap();
    fs::create_dir_all(tmp.path().join("agents/main")).unwrap();
    fs::write(tmp.path().join("agents/main/instruction.txt"), "x").unwrap();
    fs::create_dir_all(tmp.path().join("deployments/api")).unwrap();
    fs::write(
        tmp.path().join("deployments/api/deployment.yaml"),
        "channel_type: API\napp_version: \"\"\n",
    )
    .unwrap();
    let report = RuleRegistry::builtin().run_all(&discover(tmp.path()).unwrap());
    assert!(report.diagnostics.iter().any(|d| d.rule_id == "V-DEPVER"));
}
