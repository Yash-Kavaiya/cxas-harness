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

pub struct VDepverRule;

impl LintRule for VDepverRule {
    fn id(&self) -> &'static str {
        "V-DEPVER"
    }

    fn description(&self) -> &'static str {
        "deployment app_version is non-empty"
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
                    .and_then(|doc| doc.get("app_version"))
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .is_none()
            })
            .map(|dep| Diagnostic {
                rule_id: "V-DEPVER".into(),
                severity: Severity::Error,
                path: dep.path.join("deployment.yaml"),
                message: format!(
                    "deployment '{}' has a missing or empty app_version",
                    dep.name
                ),
                fix: Some("set app_version to a non-empty version or resource name".into()),
            })
            .collect()
    }
}
