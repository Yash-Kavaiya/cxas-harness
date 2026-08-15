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

fn fixture(files: &[(&str, &str)]) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    for (rel, body) in files {
        let p = tmp.path().join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }
    tmp
}

#[test]
fn missing_root_agent_is_v_root_error() {
    let dir = fixture(&[("app.yaml", "display_name: demo\n"), ("agents/main/instruction.txt", "hi")]);
    let ctx = discover(dir.path()).unwrap();
    let report = RuleRegistry::builtin().run_all(&ctx);
    assert!(report.diagnostics.iter().any(|d| d.rule_id == "V-ROOT"));
    assert!(report.error_count() >= 1);
}

#[test]
fn dangling_root_agent_is_v_root_error() {
    let dir = fixture(&[
        ("app.yaml", "display_name: demo\nroot_agent: helper\n"),
        ("agents/other/instruction.txt", "x"),
    ]);
    let ctx = discover(dir.path()).unwrap();
    let report = RuleRegistry::builtin().run_all(&ctx);
    assert!(report.diagnostics.iter().any(|d| d.rule_id == "V-ROOT"));
}

#[test]
fn valid_root_agent_is_silent() {
    let dir = fixture(&[
        ("app.yaml", "display_name: demo\nroot_agent: main\n"),
        ("agents/main/instruction.txt", "you are main"),
    ]);
    let ctx = discover(dir.path()).unwrap();
    let report = RuleRegistry::builtin().run_all(&ctx);
    assert!(!report.diagnostics.iter().any(|d| d.rule_id == "V-ROOT"));
}
