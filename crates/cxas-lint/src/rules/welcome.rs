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

use crate::context::LintContext;
use crate::diagnostic::{Diagnostic, Severity};
use crate::registry::{LintRule, RuleScope};

pub struct VWelcomeRule;

impl LintRule for VWelcomeRule {
    fn id(&self) -> &'static str {
        "V-WELCOME"
    }

    fn description(&self) -> &'static str {
        "Web Widget deployments declare a welcome_event"
    }

    fn applies_to(&self) -> RuleScope {
        RuleScope::Deployment
    }

    fn run(&self, ctx: &LintContext) -> Vec<Diagnostic> {
        ctx.deployments
            .values()
            .filter(|dep| {
                let Some(doc) = &dep.yaml else {
                    return false;
                };
                let channel = doc
                    .get("channel_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if channel != "WEB_WIDGET" {
                    return false;
                }
                doc.get("welcome_event")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .is_none()
            })
            .map(|dep| Diagnostic {
                rule_id: "V-WELCOME".into(),
                severity: Severity::Error,
                path: dep.path.join("deployment.yaml"),
                message: format!(
                    "WEB_WIDGET deployment '{}' is missing welcome_event",
                    dep.name
                ),
                fix: Some("set welcome_event on the Web Widget deployment".into()),
            })
            .collect()
    }
}
