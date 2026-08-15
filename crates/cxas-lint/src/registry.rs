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

use crate::context::LintContext;
use crate::diagnostic::{Diagnostic, LintReport};
use crate::error::LintError;
use crate::rules;

pub enum RuleScope {
    App,
    Agent,
    Tool,
    Guardrail,
    Example,
    Evaluation,
    Deployment,
}

pub trait LintRule: Send + Sync {
    fn id(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn applies_to(&self) -> RuleScope;
    fn run(&self, ctx: &LintContext) -> Vec<Diagnostic>;
}

pub struct RuleRegistry {
    rules: BTreeMap<&'static str, Box<dyn LintRule>>,
}

impl RuleRegistry {
    pub fn new() -> Self {
        Self {
            rules: BTreeMap::new(),
        }
    }

    pub fn register(&mut self, rule: Box<dyn LintRule>) {
        self.rules.insert(rule.id(), rule);
    }

    pub fn builtin() -> Self {
        let mut registry = Self::new();
        for rule in rules::builtin_rules() {
            registry.register(rule);
        }
        registry
    }

    pub fn get(&self, id: &str) -> Option<&dyn LintRule> {
        self.rules.get(id).map(|rule| rule.as_ref())
    }

    pub fn ids(&self) -> Vec<&'static str> {
        self.rules.keys().copied().collect()
    }

    pub fn run_all(&self, ctx: &LintContext) -> LintReport {
        let mut diagnostics = Vec::new();
        for rule in self.rules.values() {
            diagnostics.extend(rule.run(ctx));
        }
        LintReport { diagnostics }
    }

    pub fn run_one(&self, id: &str, ctx: &LintContext) -> Result<Vec<Diagnostic>, LintError> {
        match self.get(id) {
            Some(rule) => Ok(rule.run(ctx)),
            None => Err(LintError::UnknownRule(id.to_string())),
        }
    }
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}
