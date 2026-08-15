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

use std::path::PathBuf;

use serde_json::Value;

use crate::context::LintContext;
use crate::diagnostic::{Diagnostic, Severity};
use crate::registry::{LintRule, RuleScope};

fn error(rule_id: &str, path: PathBuf, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        rule_id: rule_id.into(),
        severity: Severity::Error,
        path,
        message: message.into(),
        fix: None,
    }
}

fn nonempty_str(value: &Value) -> Option<&str> {
    value.as_str().map(str::trim).filter(|s| !s.is_empty())
}

pub struct VSchemaAppDisplayName;

impl LintRule for VSchemaAppDisplayName {
    fn id(&self) -> &'static str {
        "V-SCHEMA-APP-DISPLAY_NAME"
    }
    fn description(&self) -> &'static str {
        "app display_name is required"
    }
    fn applies_to(&self) -> RuleScope {
        RuleScope::App
    }
    fn run(&self, ctx: &LintContext) -> Vec<Diagnostic> {
        let Some(app) = &ctx.app else {
            return Vec::new();
        };
        if app.get("display_name").and_then(nonempty_str).is_some() {
            return Vec::new();
        }
        vec![error(
            self.id(),
            ctx.root.join("app.yaml"),
            "required field display_name is missing or empty",
        )]
    }
}

pub struct VSchemaAgentInstruction;

impl LintRule for VSchemaAgentInstruction {
    fn id(&self) -> &'static str {
        "V-SCHEMA-AGENT-INSTRUCTION"
    }
    fn description(&self) -> &'static str {
        "agent instruction is required"
    }
    fn applies_to(&self) -> RuleScope {
        RuleScope::Agent
    }
    fn run(&self, ctx: &LintContext) -> Vec<Diagnostic> {
        ctx.agents
            .values()
            .filter(|agent| agent.instruction.is_none())
            .map(|agent| {
                error(
                    self.id(),
                    agent.path.join("instruction.txt"),
                    format!("agent '{}' is missing required field instruction", agent.name),
                )
            })
            .collect()
    }
}

pub struct VSchemaToolName;

impl LintRule for VSchemaToolName {
    fn id(&self) -> &'static str {
        "V-SCHEMA-TOOL-NAME"
    }
    fn description(&self) -> &'static str {
        "tool name is required"
    }
    fn applies_to(&self) -> RuleScope {
        RuleScope::Tool
    }
    fn run(&self, ctx: &LintContext) -> Vec<Diagnostic> {
        ctx.tools
            .values()
            .filter(|tool| {
                tool.yaml
                    .as_ref()
                    .and_then(|doc| doc.get("name"))
                    .and_then(nonempty_str)
                    .is_none()
            })
            .map(|tool| {
                error(
                    self.id(),
                    tool.path.join("tool.yaml"),
                    format!("tool '{}' is missing required field name", tool.name),
                )
            })
            .collect()
    }
}

pub struct VSchemaToolSchema;

impl LintRule for VSchemaToolSchema {
    fn id(&self) -> &'static str {
        "V-SCHEMA-TOOL-SCHEMA"
    }
    fn description(&self) -> &'static str {
        "tool schema is required"
    }
    fn applies_to(&self) -> RuleScope {
        RuleScope::Tool
    }
    fn run(&self, ctx: &LintContext) -> Vec<Diagnostic> {
        ctx.tools
            .values()
            .filter(|tool| tool.schema.is_none())
            .map(|tool| {
                error(
                    self.id(),
                    tool.path.join("tool.yaml"),
                    format!("tool '{}' is missing required field schema", tool.name),
                )
            })
            .collect()
    }
}

pub struct VSchemaDeploymentChannelType;

impl LintRule for VSchemaDeploymentChannelType {
    fn id(&self) -> &'static str {
        "V-SCHEMA-DEPLOYMENT-CHANNEL_TYPE"
    }
    fn description(&self) -> &'static str {
        "deployment channel_type is required"
    }
    fn applies_to(&self) -> RuleScope {
        RuleScope::Deployment
    }
    fn run(&self, ctx: &LintContext) -> Vec<Diagnostic> {
        ctx.deployments
            .values()
            .filter(|dep| {
                dep.yaml
                    .as_ref()
                    .and_then(|doc| doc.get("channel_type"))
                    .and_then(nonempty_str)
                    .is_none()
            })
            .map(|dep| {
                error(
                    self.id(),
                    dep.path.join("deployment.yaml"),
                    format!(
                        "deployment '{}' is missing required field channel_type",
                        dep.name
                    ),
                )
            })
            .collect()
    }
}

pub struct VSchemaEvaluationDisplayName;

impl LintRule for VSchemaEvaluationDisplayName {
    fn id(&self) -> &'static str {
        "V-SCHEMA-EVALUATION-DISPLAY_NAME"
    }
    fn description(&self) -> &'static str {
        "evaluation display_name is required"
    }
    fn applies_to(&self) -> RuleScope {
        RuleScope::Evaluation
    }
    fn run(&self, ctx: &LintContext) -> Vec<Diagnostic> {
        ctx.evaluations
            .iter()
            .filter(|eval| {
                eval.yaml
                    .as_ref()
                    .and_then(|doc| doc.get("display_name"))
                    .and_then(nonempty_str)
                    .is_none()
            })
            .map(|eval| {
                error(
                    self.id(),
                    eval.path.join("eval.yaml"),
                    format!(
                        "evaluation '{}' is missing required field display_name",
                        eval.name
                    ),
                )
            })
            .collect()
    }
}

pub fn schema_rules() -> Vec<Box<dyn LintRule>> {
    vec![
        Box::new(VSchemaAppDisplayName),
        Box::new(VSchemaAgentInstruction),
        Box::new(VSchemaToolName),
        Box::new(VSchemaToolSchema),
        Box::new(VSchemaDeploymentChannelType),
        Box::new(VSchemaEvaluationDisplayName),
    ]
}
