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

mod context;
mod diagnostic;
mod error;
#[cfg(feature = "llm")]
mod llm;
mod registry;
mod rules;
pub mod schema_map;

pub use context::{discover, AgentDoc, DeploymentDoc, EvalDoc, LintContext, ToolDoc};
pub use diagnostic::{Diagnostic, LintReport, Severity};
pub use error::LintError;
pub use registry::{LintRule, RuleRegistry, RuleScope};

#[cfg(feature = "llm")]
pub use llm::{InstructionFile, LlmLintClient};

#[cfg(feature = "llm")]
pub mod test_support {
    pub use crate::llm::spawn_json_listener;
}

pub fn crate_name() -> &'static str {
    "cxas-lint"
}
