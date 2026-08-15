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

/// `(section, field, rule_id)` mappings for required schema fields.
pub const FIELD_RULES: &[(&str, &str, &str)] = &[
    ("app", "display_name", "V-SCHEMA-APP-DISPLAY_NAME"),
    ("app", "root_agent", "V-ROOT"),
    ("agent", "instruction", "V-SCHEMA-AGENT-INSTRUCTION"),
    ("tool", "name", "V-SCHEMA-TOOL-NAME"),
    ("tool", "schema", "V-SCHEMA-TOOL-SCHEMA"),
    ("deployment", "channel_type", "V-SCHEMA-DEPLOYMENT-CHANNEL_TYPE"),
    ("evaluation", "display_name", "V-SCHEMA-EVALUATION-DISPLAY_NAME"),
];

pub fn rule_id_for(section: &str, field: &str) -> Option<&'static str> {
    FIELD_RULES
        .iter()
        .find(|(s, f, _)| *s == section && *f == field)
        .map(|(_, _, id)| *id)
}

/// Writes a complete valid app, then removes only `section.field`.
pub fn fixture_omitting(section: &str, field: &str) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    let app = match (section, field) {
        ("app", "display_name") => "root_agent: main\n",
        ("app", "root_agent") => "display_name: demo\n",
        _ => "display_name: demo\nroot_agent: main\n",
    };
    fs::write(root.join("app.yaml"), app).unwrap();

    fs::create_dir_all(root.join("agents/main")).unwrap();
    if section == "agent" && field == "instruction" {
        fs::write(root.join("agents/main/agent.yaml"), "display_name: main\n").unwrap();
    } else {
        fs::write(root.join("agents/main/instruction.txt"), "you are main").unwrap();
    }

    fs::create_dir_all(root.join("tools/search")).unwrap();
    let tool = match (section, field) {
        ("tool", "name") => "schema:\n  type: object\n  properties: {}\n",
        ("tool", "schema") => "name: search\n",
        _ => "name: search\nschema:\n  type: object\n  properties: {}\n",
    };
    fs::write(root.join("tools/search/tool.yaml"), tool).unwrap();

    fs::create_dir_all(root.join("deployments/web")).unwrap();
    let deployment = if section == "deployment" && field == "channel_type" {
        "app_version: v1\nwelcome_event: hello\n"
    } else {
        "channel_type: WEB_WIDGET\napp_version: v1\nwelcome_event: hello\n"
    };
    fs::write(root.join("deployments/web/deployment.yaml"), deployment).unwrap();

    fs::create_dir_all(root.join("evaluations/basic")).unwrap();
    let evaluation = if section == "evaluation" && field == "display_name" {
        "agent: main\n"
    } else {
        "display_name: basic\nagent: main\n"
    };
    fs::write(root.join("evaluations/basic/eval.yaml"), evaluation).unwrap();

    tmp
}
