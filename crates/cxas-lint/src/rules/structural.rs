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
use std::path::PathBuf;

use serde_json::Value;

use crate::context::LintContext;
use crate::diagnostic::{Diagnostic, Severity};
use crate::registry::{LintRule, RuleScope};

fn diagnostic(
    rule_id: &str,
    severity: Severity,
    path: PathBuf,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        rule_id: rule_id.into(),
        severity,
        path,
        message: message.into(),
        fix: None,
    }
}

fn nonempty_str(value: &Value) -> Option<&str> {
    value.as_str().map(str::trim).filter(|s| !s.is_empty())
}

pub struct V001;

impl LintRule for V001 {
    fn id(&self) -> &'static str {
        "V001"
    }
    fn description(&self) -> &'static str {
        "app.yaml or app.json exists at the app root"
    }
    fn applies_to(&self) -> RuleScope {
        RuleScope::App
    }
    fn run(&self, ctx: &LintContext) -> Vec<Diagnostic> {
        let yaml = ctx.root.join("app.yaml");
        let yml = ctx.root.join("app.yml");
        let json = ctx.root.join("app.json");
        if yaml.is_file() || yml.is_file() || json.is_file() {
            Vec::new()
        } else {
            vec![diagnostic(
                self.id(),
                Severity::Error,
                yaml,
                "app.yaml or app.json must exist at the app root",
            )]
        }
    }
}

pub struct V002;

impl LintRule for V002 {
    fn id(&self) -> &'static str {
        "V002"
    }
    fn description(&self) -> &'static str {
        "display_name is non-empty"
    }
    fn applies_to(&self) -> RuleScope {
        RuleScope::App
    }
    fn run(&self, ctx: &LintContext) -> Vec<Diagnostic> {
        let Some(app) = &ctx.app else {
            return Vec::new();
        };
        match app.get("display_name").and_then(nonempty_str) {
            Some(_) => Vec::new(),
            None => vec![diagnostic(
                self.id(),
                Severity::Error,
                ctx.root.join("app.yaml"),
                "display_name is empty or missing",
            )],
        }
    }
}

pub struct V003;

impl LintRule for V003 {
    fn id(&self) -> &'static str {
        "V003"
    }
    fn description(&self) -> &'static str {
        "every agents/<name>/ directory contains instruction.txt or agent.yaml"
    }
    fn applies_to(&self) -> RuleScope {
        RuleScope::Agent
    }
    fn run(&self, ctx: &LintContext) -> Vec<Diagnostic> {
        ctx.agents
            .values()
            .filter(|agent| {
                !agent.path.join("instruction.txt").is_file()
                    && !agent.path.join("agent.yaml").is_file()
                    && !agent.path.join("agent.yml").is_file()
                    && !agent.path.join("agent.json").is_file()
            })
            .map(|agent| {
                diagnostic(
                    self.id(),
                    Severity::Error,
                    agent.path.clone(),
                    format!(
                        "agent '{}' is missing instruction.txt or agent.yaml",
                        agent.name
                    ),
                )
            })
            .collect()
    }
}

pub struct V004;

impl LintRule for V004 {
    fn id(&self) -> &'static str {
        "V004"
    }
    fn description(&self) -> &'static str {
        "every tool referenced by an agent exists under tools/"
    }
    fn applies_to(&self) -> RuleScope {
        RuleScope::Tool
    }
    fn run(&self, ctx: &LintContext) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        for agent in ctx.agents.values() {
            let Some(tools) = agent.yaml.as_ref().and_then(|doc| doc.get("tools")) else {
                continue;
            };
            let names = match tools {
                Value::Array(items) => items
                    .iter()
                    .filter_map(|item| {
                        item.as_str()
                            .or_else(|| item.get("name").and_then(Value::as_str))
                    })
                    .collect::<Vec<_>>(),
                Value::String(name) => vec![name.as_str()],
                _ => Vec::new(),
            };
            for name in names {
                if !ctx.tools.contains_key(name) {
                    out.push(diagnostic(
                        self.id(),
                        Severity::Error,
                        agent.path.clone(),
                        format!("agent '{}' references missing tool '{name}'", agent.name),
                    ));
                }
            }
        }
        out
    }
}

pub struct V005;

impl LintRule for V005 {
    fn id(&self) -> &'static str {
        "V005"
    }
    fn description(&self) -> &'static str {
        "tool schema JSON is a valid JSON Schema object (draft 2020-12 shape)"
    }
    fn applies_to(&self) -> RuleScope {
        RuleScope::Tool
    }
    fn run(&self, ctx: &LintContext) -> Vec<Diagnostic> {
        ctx.tools
            .values()
            .filter_map(|tool| {
                let schema = tool.schema.as_ref()?;
                if schema.is_object() {
                    None
                } else {
                    Some(diagnostic(
                        self.id(),
                        Severity::Error,
                        tool.path.join("tool.yaml"),
                        format!(
                            "tool '{}' schema must be a JSON object (JSON Schema draft 2020-12)",
                            tool.name
                        ),
                    ))
                }
            })
            .collect()
    }
}

pub struct V006;

impl LintRule for V006 {
    fn id(&self) -> &'static str {
        "V006"
    }
    fn description(&self) -> &'static str {
        "evaluation files reference existing agents"
    }
    fn applies_to(&self) -> RuleScope {
        RuleScope::Evaluation
    }
    fn run(&self, ctx: &LintContext) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        for eval in &ctx.evaluations {
            let Some(doc) = &eval.yaml else {
                continue;
            };
            let mut names = Vec::new();
            if let Some(name) = doc.get("agent").and_then(Value::as_str) {
                names.push(name);
            }
            if let Some(name) = doc.get("root_agent").and_then(Value::as_str) {
                names.push(name);
            }
            if let Some(Value::Array(items)) = doc.get("agents") {
                names.extend(items.iter().filter_map(Value::as_str));
            }
            for name in names {
                if !ctx.agents.contains_key(name) {
                    out.push(diagnostic(
                        self.id(),
                        Severity::Error,
                        eval.path.clone(),
                        format!(
                            "evaluation '{}' references missing agent '{name}'",
                            eval.name
                        ),
                    ));
                }
            }
        }
        out
    }
}

pub struct V007;

impl LintRule for V007 {
    fn id(&self) -> &'static str {
        "V007"
    }
    fn description(&self) -> &'static str {
        "instruction.txt must not be empty when present"
    }
    fn applies_to(&self) -> RuleScope {
        RuleScope::Agent
    }
    fn run(&self, ctx: &LintContext) -> Vec<Diagnostic> {
        ctx.agents
            .values()
            .filter_map(|agent| {
                let path = agent.path.join("instruction.txt");
                if !path.is_file() {
                    return None;
                }
                let body = std::fs::read_to_string(&path).unwrap_or_default();
                if body.trim().is_empty() {
                    Some(diagnostic(
                        self.id(),
                        Severity::Error,
                        path,
                        format!("agent '{}' has an empty instruction.txt", agent.name),
                    ))
                } else {
                    None
                }
            })
            .collect()
    }
}

pub struct V008;

impl LintRule for V008 {
    fn id(&self) -> &'static str {
        "V008"
    }
    fn description(&self) -> &'static str {
        "agent display names must be unique"
    }
    fn applies_to(&self) -> RuleScope {
        RuleScope::Agent
    }
    fn run(&self, ctx: &LintContext) -> Vec<Diagnostic> {
        let mut seen: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for agent in ctx.agents.values() {
            if let Some(name) = agent
                .yaml
                .as_ref()
                .and_then(|doc| doc.get("display_name"))
                .and_then(nonempty_str)
            {
                seen.entry(name).or_default().push(agent.name.as_str());
            }
        }
        seen.into_iter()
            .filter(|(_, agents)| agents.len() > 1)
            .map(|(display, agents)| {
                diagnostic(
                    self.id(),
                    Severity::Error,
                    ctx.root.join("agents"),
                    format!("duplicate agent display_name '{display}' on {}", agents.join(", ")),
                )
            })
            .collect()
    }
}

pub struct V009;

impl LintRule for V009 {
    fn id(&self) -> &'static str {
        "V009"
    }
    fn description(&self) -> &'static str {
        "environment.json boolean fields must be JSON booleans, not strings"
    }
    fn applies_to(&self) -> RuleScope {
        RuleScope::App
    }
    fn run(&self, ctx: &LintContext) -> Vec<Diagnostic> {
        let path = ctx.root.join("environment.json");
        if !path.is_file() {
            return Vec::new();
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            return vec![diagnostic(
                self.id(),
                Severity::Error,
                path,
                "environment.json is unreadable",
            )];
        };
        let Ok(value) = serde_json::from_str::<Value>(&text) else {
            return vec![diagnostic(
                self.id(),
                Severity::Error,
                path,
                "environment.json is not valid JSON",
            )];
        };
        let Some(obj) = value.as_object() else {
            return vec![diagnostic(
                self.id(),
                Severity::Error,
                path,
                "environment.json must be a JSON object",
            )];
        };
        obj.iter()
            .filter(|(k, v)| {
                matches!(k.as_str(), "debug" | "enabled" | "tracing") && v.is_string()
            })
            .map(|(k, _)| {
                diagnostic(
                    self.id(),
                    Severity::Error,
                    path.clone(),
                    format!("environment.json field '{k}' must be a boolean, not a string"),
                )
            })
            .collect()
    }
}

/// Optional-file Info rule with a unique id and relative path.
pub struct OptionalFileRule {
    id: &'static str,
    rel: &'static str,
    description: &'static str,
}

impl LintRule for OptionalFileRule {
    fn id(&self) -> &'static str {
        self.id
    }
    fn description(&self) -> &'static str {
        self.description
    }
    fn applies_to(&self) -> RuleScope {
        RuleScope::App
    }
    fn run(&self, ctx: &LintContext) -> Vec<Diagnostic> {
        let path = ctx.root.join(self.rel);
        if path.exists() {
            Vec::new()
        } else {
            vec![diagnostic(
                self.id(),
                Severity::Info,
                path,
                format!("optional file {} is absent", self.rel),
            )]
        }
    }
}

/// Unknown-key warning for a distinct unused app key.
pub struct UnusedKeyRule {
    id: &'static str,
    key: &'static str,
    description: &'static str,
}

impl LintRule for UnusedKeyRule {
    fn id(&self) -> &'static str {
        self.id
    }
    fn description(&self) -> &'static str {
        self.description
    }
    fn applies_to(&self) -> RuleScope {
        RuleScope::App
    }
    fn run(&self, ctx: &LintContext) -> Vec<Diagnostic> {
        let Some(app) = &ctx.app else {
            return Vec::new();
        };
        if app.get(self.key).is_some() {
            vec![diagnostic(
                self.id(),
                Severity::Warning,
                ctx.root.join("app.yaml"),
                format!("unknown app key '{}'", self.key),
            )]
        } else {
            Vec::new()
        }
    }
}

const OPTIONAL_FILES: &[(&str, &str, &str)] = &[
    ("V010", "README.md", "optional README.md"),
    ("V011", "CHANGELOG.md", "optional CHANGELOG.md"),
    ("V012", "LICENSE", "optional LICENSE"),
    ("V013", ".gitignore", "optional .gitignore"),
    ("V014", "global_instruction.txt", "optional global_instruction.txt"),
    ("V015", "environment.json", "optional environment.json"),
    ("V016", "examples.yaml", "optional examples.yaml"),
    ("V017", "guardrails.yaml", "optional guardrails.yaml"),
    ("V018", "callbacks.yaml", "optional callbacks.yaml"),
    ("V019", "versions/.gitkeep", "optional versions/.gitkeep"),
    ("V020", "docs/index.md", "optional docs/index.md"),
    ("V021", "playbooks/default.yaml", "optional playbooks/default.yaml"),
    ("V022", "skills/manifest.yaml", "optional skills/manifest.yaml"),
    ("V023", "tests/golden.yaml", "optional tests/golden.yaml"),
    ("V024", "data/dataset.jsonl", "optional data/dataset.jsonl"),
    ("V025", "prompts/system.txt", "optional prompts/system.txt"),
    ("V026", "config/logging.yaml", "optional config/logging.yaml"),
    ("V027", "config/metrics.yaml", "optional config/metrics.yaml"),
    ("V028", "secrets.example.yaml", "optional secrets.example.yaml"),
    ("V029", "Makefile", "optional Makefile"),
];

const UNUSED_KEYS: &[(&str, &str, &str)] = &[
    ("V030", "legacy_name", "unknown app key legacy_name"),
    ("V031", "deprecated_root", "unknown app key deprecated_root"),
    ("V032", "scratchpad", "unknown app key scratchpad"),
    ("V033", "internal_only", "unknown app key internal_only"),
    ("V034", "tmp_flag", "unknown app key tmp_flag"),
    ("V035", "experimental_ui", "unknown app key experimental_ui"),
    ("V036", "old_display", "unknown app key old_display"),
    ("V037", "shadow_agent", "unknown app key shadow_agent"),
    ("V038", "debug_hooks", "unknown app key debug_hooks"),
    ("V039", "unused_channel", "unknown app key unused_channel"),
    ("V040", "beta_tools", "unknown app key beta_tools"),
    ("V041", "hidden_eval", "unknown app key hidden_eval"),
    ("V042", "placeholder_icon", "unknown app key placeholder_icon"),
    ("V043", "draft_version", "unknown app key draft_version"),
    ("V044", "orphan_resource", "unknown app key orphan_resource"),
    ("V045", "stale_ref", "unknown app key stale_ref"),
    ("V046", "tmp_schema", "unknown app key tmp_schema"),
    ("V047", "unused_model", "unknown app key unused_model"),
    ("V048", "legacy_welcome", "unknown app key legacy_welcome"),
    ("V049", "ghost_deployment", "unknown app key ghost_deployment"),
    ("V050", "unused_guardrail", "unknown app key unused_guardrail"),
    ("V051", "scratch_eval", "unknown app key scratch_eval"),
    ("V052", "draft_tool", "unknown app key draft_tool"),
    ("V053", "tmp_example", "unknown app key tmp_example"),
    ("V054", "legacy_callback", "unknown app key legacy_callback"),
    ("V055", "unused_dataset", "unknown app key unused_dataset"),
    ("V056", "stale_playbook", "unknown app key stale_playbook"),
    ("V057", "hidden_skill", "unknown app key hidden_skill"),
    ("V058", "orphan_version", "unknown app key orphan_version"),
    ("V059", "tmp_prompt", "unknown app key tmp_prompt"),
    ("V060", "unused_metric", "unknown app key unused_metric"),
];

pub fn structural_rules() -> Vec<Box<dyn LintRule>> {
    let mut rules: Vec<Box<dyn LintRule>> = vec![
        Box::new(V001),
        Box::new(V002),
        Box::new(V003),
        Box::new(V004),
        Box::new(V005),
        Box::new(V006),
        Box::new(V007),
        Box::new(V008),
        Box::new(V009),
    ];
    for (id, rel, description) in OPTIONAL_FILES {
        rules.push(Box::new(OptionalFileRule {
            id,
            rel,
            description,
        }));
    }
    for (id, key, description) in UNUSED_KEYS {
        rules.push(Box::new(UnusedKeyRule {
            id,
            key,
            description,
        }));
    }
    rules
}
