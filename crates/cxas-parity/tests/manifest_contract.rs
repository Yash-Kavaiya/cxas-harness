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

//! Python `cxas-scrapi` surface reference — **CLI shape only**.
//!
//! These assertions describe what users expect the CLI to look like. They are
//! NOT a correctness benchmark: the manifest is hand-written, so it asserts
//! only that a checked-in file contains strings that same file declares.
//! The API-correctness contract lives in `discovery_contract.rs`, which asserts
//! against Google's vendored discovery documents. Where the two disagree,
//! discovery wins.

use cxas_parity::{load_bundled, ParityError, ParityManifest};

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
fn python_surface_declares_every_frozen_class() {
    let m = load_bundled().unwrap();
    for class in REQUIRED_CLASSES {
        m.require_type(class)
            .unwrap_or_else(|_| panic!("missing class {class}"));
    }
}

const REQUIRED_METHODS: &[(&str, &[&str])] = &[
    (
        "Apps",
        &[
            "list_apps",
            "get_app",
            "get_app_by_display_name",
            "create_app",
            "delete_app",
            "export_app",
            "import_app",
            "import_as_new_app",
            "get_apps_map",
        ],
    ),
    (
        "Agents",
        &[
            "get_agents_map",
            "list_agents",
            "get_agent",
            "create_agent",
            "update_agent",
            "delete_agent",
        ],
    ),
    (
        "Tools",
        &[
            "get_tools_map",
            "list_tools",
            "get_tool",
            "create_tool",
            "update_tool",
            "delete_tool",
        ],
    ),
    (
        "Guardrails",
        &[
            "list_guardrails",
            "get_guardrail",
            "create_guardrail",
            "update_guardrail",
            "delete_guardrail",
        ],
    ),
    (
        "Deployments",
        &[
            "list_deployments",
            "get_deployment",
            "create_deployment",
            "update_deployment",
            "delete_deployment",
        ],
    ),
    (
        "Sessions",
        &["create_session_id", "run", "parse_result", "bidi_run"],
    ),
    (
        "Evaluations",
        &[
            "list_evaluations",
            "get_evaluation",
            "update_evaluation",
            "run_evaluation",
            "export_evaluation",
            "get_evaluation_result",
            "wait_for_run_and_get_results",
            "get_evaluations_map",
        ],
    ),
    ("SimulationEvals", &["run_simulations"]),
    (
        "Versions",
        &[
            "list_versions",
            "create_version",
            "compare_versions",
            "get_version",
        ],
    ),
];

#[test]
fn python_surface_declares_method_minima() {
    let m = load_bundled().unwrap();
    for (class, methods) in REQUIRED_METHODS {
        let ty = m
            .require_type(class)
            .unwrap_or_else(|_| panic!("missing class {class}"));
        for name in *methods {
            assert!(
                ty.methods.iter().any(|mm| mm.name == *name),
                "{class} missing method {name}"
            );
        }
    }
}

const REQUIRED_CLI: &[&[&str]] = &[
    &["migrate", "dfcx"],
    &["init-github-action"],
    &["evals", "report"],
    &["test-tools"],
    &["test-callbacks"],
    &["test-single-callback"],
    &["export"],
    &["push-eval"],
    &["run"],
    &["run-session"],
    &["ci-test"],
    &["local-test"],
    &["delete"],
    &["pull"],
    &["push"],
    &["lint"],
    &["llm-lint"],
    &["help"],
    &["init"],
    &["create"],
    &["branch"],
    &["apps", "list"],
    &["apps", "get"],
    &["conversations", "list"],
    &["conversations", "get"],
    &["deployments", "list"],
    &["deployments", "create"],
    &["deployments", "promote"],
    &["local", "create"],
    &["versions", "list"],
    &["versions", "compare"],
    &["insights"],
    &["trace"],
    &["agent"],
    &["tool"],
    &["guardrail"],
];

#[test]
fn python_surface_declares_cli_commands() {
    let m = load_bundled().unwrap();
    for argv in REQUIRED_CLI {
        m.require_command(argv)
            .unwrap_or_else(|_| panic!("missing {argv:?}"));
    }
}

#[test]
fn json_round_trip_equals_bundled() {
    let original = load_bundled().expect("bundled YAML must parse");
    let json = original.to_json().expect("to_json");
    let parsed: ParityManifest = serde_json::from_str(&json).expect("JSON must parse");
    assert_eq!(original, parsed);
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
