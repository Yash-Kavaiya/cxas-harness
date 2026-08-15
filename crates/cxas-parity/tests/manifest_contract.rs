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

use cxas_parity::{load_bundled, ParityError};

#[test]
fn bundled_manifest_loads_and_has_version_1() {
    let m = load_bundled().expect("bundled YAML must parse");
    assert_eq!(m.version, 1);
    assert_eq!(m.source.commit, "4f7b43ca6adda0acad95a7e3654eee4e2ed1438c");
}

#[test]
fn missing_file_is_io_error() {
    let err = cxas_parity::load_manifest(std::path::Path::new("this/path/does/not/exist.yaml"))
        .unwrap_err();
    assert!(matches!(err, ParityError::Io(_)));
}

const REQUIRED_CLASSES: &[&str] = &[
    "Agents",
    "Apps",
    "Callbacks",
    "Changelogs",
    "Common",
    "ConversationHistory",
    "Deployments",
    "Evaluations",
    "Guardrails",
    "Sessions",
    "Tools",
    "Variables",
    "Versions",
    "CallbackEvals",
    "GuardrailEvals",
    "SimulationEvals",
    "ToolEvals",
    "TurnEvals",
    "EvalUtils",
    "GoogleSheetsUtils",
    "SecretManagerUtils",
    "ChangelogUtils",
    "BaseDFCXClient",
    "ConversationalAgentsAPI",
    "DFCXAgentExporter",
    "DFCXAgents",
    "DFCXGenerativeSettings",
    "DFCXPlaybooks",
    "DFCXTools",
    "FlowDependencyResolver",
    "FlowTreeVisualizer",
    "HighLevelGraphVisualizer",
    "MainVisualizer",
    "PlaybookTreeVisualizer",
];

#[test]
fn every_frozen_python_class_is_present() {
    let m = load_bundled().unwrap();
    for class in REQUIRED_CLASSES {
        m.require_type(class)
            .unwrap_or_else(|_| panic!("missing class {class}"));
    }
}

#[test]
fn apps_sessions_evaluations_have_required_methods() {
    let m = load_bundled().unwrap();
    let apps = m.require_type("Apps").unwrap();
    for name in [
        "list_apps",
        "get_app",
        "export_app",
        "import_app",
        "import_as_new_app",
    ] {
        assert!(apps.methods.iter().any(|mm| mm.name == name), "{name}");
    }
    let evals = m.require_type("Evaluations").unwrap();
    assert!(evals
        .methods
        .iter()
        .any(|mm| mm.name == "wait_for_run_and_get_results"));
}

#[test]
fn frozen_cli_commands_are_present() {
    let m = load_bundled().unwrap();
    for argv in [
        &["pull"][..],
        &["push"],
        &["lint"],
        &["llm-lint"],
        &["evals", "report"],
        &["migrate", "dfcx"],
        &["trace"],
        &["init-github-action"],
    ] {
        m.require_command(argv)
            .unwrap_or_else(|_| panic!("missing {argv:?}"));
    }
}

#[test]
fn issue_gate_284_is_declared() {
    let m = load_bundled().unwrap();
    assert!(m
        .issue_gates()
        .iter()
        .any(|g| g.id == 284 && g.crate_name == "cxas-proto"));
}

#[test]
fn duplicate_class_is_rejected() {
    let yaml = r#"
version: 1
source: { repository: x, commit: y }
modules:
  - name: a
    rust_owner: cxas-core
    types:
      - { python_class: Apps, python_module: m, rust_type: Apps, methods: [] }
      - { python_class: Apps, python_module: m, rust_type: Apps, methods: [] }
enums: []
cli: { binary: cxas, commands: [] }
issue_gates: []
"#;
    let err = cxas_parity::parse_yaml_for_test(yaml).unwrap_err();
    assert!(matches!(err, ParityError::Duplicate(_)));
}
