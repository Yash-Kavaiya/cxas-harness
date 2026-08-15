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

use crate::ParityError;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub repository: String,
    pub commit: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParityType {
    pub python_class: String,
    pub python_module: String,
    pub rust_type: String,
    pub methods: Vec<ParityMethod>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParityMethod {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParityModule {
    pub name: String,
    pub rust_owner: String,
    #[serde(default)]
    pub types: Vec<ParityType>,
    #[serde(default)]
    pub commands: Vec<ParityCommand>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParityCommand {
    pub argv: Vec<String>,
    pub python_handler: String,
    pub rust_owner: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParityEnum {
    pub python_name: String,
    pub proto_type: String,
    pub rust_type: String,
    pub rust_owner: String,
    pub unknown_policy: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IssueGate {
    pub id: u32,
    pub crate_name: String,
    pub test: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cli {
    pub binary: String,
    #[serde(default)]
    pub global_flags: Vec<String>,
    #[serde(default)]
    pub commands: Vec<ParityCommand>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParityManifest {
    pub version: u32,
    pub source: Source,
    pub modules: Vec<ParityModule>,
    pub enums: Vec<ParityEnum>,
    pub cli: Cli,
    pub issue_gates: Vec<IssueGate>,
}

const BUNDLED: &str = include_str!("../../../parity/cxas-scrapi-parity.yaml");

impl ParityManifest {
    pub fn require_type(&self, python_class: &str) -> Result<&ParityType, ParityError> {
        self.modules
            .iter()
            .flat_map(|m| m.types.iter())
            .find(|t| t.python_class == python_class)
            .ok_or_else(|| ParityError::Missing(python_class.into()))
    }

    pub fn require_command(&self, argv: &[&str]) -> Result<&ParityCommand, ParityError> {
        let wanted: Vec<String> = argv.iter().map(|s| (*s).to_string()).collect();
        self.cli
            .commands
            .iter()
            .find(|c| c.argv == wanted)
            .ok_or_else(|| ParityError::Missing(wanted.join(" ")))
    }

    pub fn types_for_crate(&self, rust_owner: &str) -> Vec<&ParityType> {
        self.modules
            .iter()
            .filter(|m| m.rust_owner == rust_owner)
            .flat_map(|m| m.types.iter())
            .collect()
    }

    pub fn commands_for_crate(&self, rust_owner: &str) -> Vec<&ParityCommand> {
        self.cli
            .commands
            .iter()
            .filter(|c| c.rust_owner == rust_owner)
            .collect()
    }

    pub fn issue_gates(&self) -> &[IssueGate] {
        &self.issue_gates
    }

    pub fn to_json(&self) -> Result<String, ParityError> {
        serde_json::to_string_pretty(self).map_err(|e| ParityError::Schema(e.to_string()))
    }
}

pub fn load_manifest(path: &Path) -> Result<ParityManifest, ParityError> {
    let text = std::fs::read_to_string(path)?;
    parse_yaml(&text)
}

pub fn load_bundled() -> Result<ParityManifest, ParityError> {
    parse_yaml(BUNDLED)
}

fn parse_yaml(text: &str) -> Result<ParityManifest, ParityError> {
    let m: ParityManifest = serde_yaml::from_str(text)?;
    if m.version != 1 {
        return Err(ParityError::Schema(format!("version {} != 1", m.version)));
    }
    Ok(m)
}
