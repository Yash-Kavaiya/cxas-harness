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

use crate::context::LintContext;
use crate::diagnostic::{Diagnostic, Severity};
use crate::registry::{LintRule, RuleScope};

pub struct VRootRule;

impl LintRule for VRootRule {
    fn id(&self) -> &'static str {
        "V-ROOT"
    }

    fn description(&self) -> &'static str {
        "app.yaml / app.json names a root_agent (or start_agent) that exists under agents/"
    }

    fn applies_to(&self) -> RuleScope {
        RuleScope::App
    }

    fn run(&self, ctx: &LintContext) -> Vec<Diagnostic> {
        let Some(app) = &ctx.app else {
            return Vec::new();
        };
        let name = app
            .get("root_agent")
            .or_else(|| app.get("start_agent"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());
        match name {
            None => vec![v_root(
                ctx,
                "missing root_agent (or start_agent) that names an existing agents/ directory",
            )],
            Some(agent) if !ctx.agents.contains_key(agent) => vec![v_root(
                ctx,
                &format!("root_agent '{agent}' does not exist under agents/"),
            )],
            Some(_) => Vec::new(),
        }
    }
}

fn v_root(ctx: &LintContext, message: &str) -> Diagnostic {
    let path = ["app.yaml", "app.yml", "app.json"]
        .into_iter()
        .map(|name| ctx.root.join(name))
        .find(|p| p.is_file())
        .unwrap_or_else(|| PathBuf::from("app.yaml"));
    Diagnostic {
        rule_id: "V-ROOT".into(),
        severity: Severity::Error,
        path,
        message: message.into(),
        fix: Some("set root_agent to an existing agents/<name> directory".into()),
    }
}
