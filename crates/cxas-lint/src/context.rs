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

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::LintError;

#[derive(Debug, Clone)]
pub struct AgentDoc {
    pub name: String,
    pub path: PathBuf,
    pub instruction: Option<String>,
    pub yaml: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct ToolDoc {
    pub name: String,
    pub path: PathBuf,
    pub yaml: Option<Value>,
    pub schema: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct DeploymentDoc {
    pub name: String,
    pub path: PathBuf,
    pub yaml: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct EvalDoc {
    pub name: String,
    pub path: PathBuf,
    pub yaml: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct LintContext {
    pub root: PathBuf,
    pub app: Option<Value>,
    pub agents: BTreeMap<String, AgentDoc>,
    pub tools: BTreeMap<String, ToolDoc>,
    pub deployments: BTreeMap<String, DeploymentDoc>,
    pub evaluations: Vec<EvalDoc>,
}

pub fn discover(root: &Path) -> Result<LintContext, LintError> {
    let meta = fs::metadata(root)?;
    if !meta.is_dir() {
        return Err(LintError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("app dir is not a directory: {}", root.display()),
        )));
    }

    Ok(LintContext {
        root: root.to_path_buf(),
        app: read_app(root),
        agents: read_agents(root),
        tools: read_tools(root),
        deployments: read_deployments(root),
        evaluations: read_evaluations(root),
    })
}

fn read_app(root: &Path) -> Option<Value> {
    for name in ["app.yaml", "app.yml", "app.json"] {
        let path = root.join(name);
        if path.is_file() {
            return parse_doc(&path);
        }
    }
    None
}

fn read_agents(root: &Path) -> BTreeMap<String, AgentDoc> {
    let mut agents = BTreeMap::new();
    let dir = root.join("agents");
    for name in list_dirs(&dir) {
        let path = dir.join(&name);
        let yaml = first_doc(&path, &["agent.yaml", "agent.yml", "agent.json"]);
        let instruction = read_instruction(&path, yaml.as_ref());
        agents.insert(
            name.clone(),
            AgentDoc {
                name,
                path,
                instruction,
                yaml,
            },
        );
    }
    agents
}

fn read_instruction(agent_dir: &Path, yaml: Option<&Value>) -> Option<String> {
    let from_file = fs::read_to_string(agent_dir.join("instruction.txt"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if from_file.is_some() {
        return from_file;
    }
    yaml.and_then(|doc| {
        doc.get("instruction")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    })
}

fn read_tools(root: &Path) -> BTreeMap<String, ToolDoc> {
    let mut tools = BTreeMap::new();
    let dir = root.join("tools");
    for name in list_dirs(&dir) {
        let path = dir.join(&name);
        let yaml = first_doc(&path, &["tool.yaml", "tool.yml", "tool.json"]);
        let schema = yaml
            .as_ref()
            .and_then(|doc| doc.get("schema").cloned())
            .or_else(|| first_doc(&path, &["schema.json", "schema.yaml", "schema.yml"]));
        tools.insert(
            name.clone(),
            ToolDoc {
                name,
                path,
                yaml,
                schema,
            },
        );
    }
    tools
}

fn read_deployments(root: &Path) -> BTreeMap<String, DeploymentDoc> {
    let mut deployments = BTreeMap::new();
    let dir = root.join("deployments");
    for name in list_dirs(&dir) {
        let path = dir.join(&name);
        let yaml = first_doc(
            &path,
            &[
                "deployment.yaml",
                "deployment.yml",
                "deployment.json",
            ],
        );
        deployments.insert(
            name.clone(),
            DeploymentDoc {
                name,
                path,
                yaml,
            },
        );
    }
    deployments
}

fn read_evaluations(root: &Path) -> Vec<EvalDoc> {
    let mut evaluations = Vec::new();
    let dir = root.join("evaluations");
    for name in list_dirs(&dir) {
        let path = dir.join(&name);
        let yaml = first_doc(
            &path,
            &[
                "eval.yaml",
                "eval.yml",
                "evaluation.yaml",
                "evaluation.yml",
                "eval.json",
                "evaluation.json",
            ],
        );
        evaluations.push(EvalDoc { name, path, yaml });
    }
    evaluations
}

fn list_dirs(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                if !name.starts_with('.') {
                    names.push(name.to_string());
                }
            }
        }
    }
    names.sort();
    names
}

fn first_doc(dir: &Path, names: &[&str]) -> Option<Value> {
    for name in names {
        let path = dir.join(name);
        if path.is_file() {
            if let Some(doc) = parse_doc(&path) {
                return Some(doc);
            }
        }
    }
    None
}

fn parse_doc(path: &Path) -> Option<Value> {
    let text = fs::read_to_string(path).ok()?;
    if path.extension().and_then(|e| e.to_str()) == Some("json") {
        serde_json::from_str(&text).ok()
    } else {
        let yaml: serde_yaml::Value = serde_yaml::from_str(&text).ok()?;
        serde_json::to_value(yaml).ok()
    }
}
