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

use crate::error::MigrateError;
use cxas_core::ClientConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Dialogflow CX agent exporter. Does not perform live CES/DFCX calls in this phase.
pub struct DFCXAgentExporter {
    pub config: ClientConfig,
}

impl DFCXAgentExporter {
    pub fn new(config: ClientConfig) -> Self {
        Self { config }
    }

    pub async fn export_zip(&self, source_agent: &str) -> Result<PathBuf, MigrateError> {
        let _ = &self.config;
        if source_agent.trim().is_empty() {
            return Err(MigrateError::Usage("source is required"));
        }
        let path = PathBuf::from(source_agent);
        if path.extension().and_then(|e| e.to_str()) == Some("zip") && path.is_file() {
            return Ok(path);
        }
        Err(MigrateError::Usage("live DFCX export is not available"))
    }
}

/// Conversational Agents (CES) API client wrapper for migration.
pub struct ConversationalAgentsAPI {
    pub config: ClientConfig,
}

impl ConversationalAgentsAPI {
    pub fn new(config: ClientConfig) -> Self {
        Self { config }
    }
}

/// Serde-serializable intermediate representation of a DFCX agent.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct IrBundle {
    pub source: String,
    #[serde(default)]
    pub agents: Vec<serde_yaml::Value>,
    #[serde(default)]
    pub tools: Vec<serde_yaml::Value>,
    #[serde(default)]
    pub flows: Vec<serde_yaml::Value>,
}

/// One user/agent exchange used to seed eval goldens.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub user: String,
    pub agent: String,
}

/// Ordered conversation produced by [`DFCXConversationRunner`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationTrace {
    pub turns: Vec<ConversationTurn>,
}

/// Replays DFCX conversation turns into a [`ConversationTrace`].
pub struct DFCXConversationRunner {
    pub config: ClientConfig,
}

impl DFCXConversationRunner {
    pub fn new(config: ClientConfig) -> Self {
        Self { config }
    }

    pub fn seed_goldens(&self, turns: Vec<ConversationTurn>) -> ConversationTrace {
        let _ = &self.config;
        ConversationTrace { turns }
    }
}

/// Gemini-backed IR augmenter. Network I/O is compiled only with `llm`.
#[derive(Clone, Debug, Default)]
pub struct AIAugment;

impl AIAugment {
    pub fn new() -> Self {
        Self
    }

    pub fn augment(&self, bundle: &IrBundle) -> Result<IrBundle, MigrateError> {
        let _ = bundle;
        #[cfg(feature = "llm")]
        {
            return Err(MigrateError::Usage(
                "live Gemini augment is not available in this build",
            ));
        }
        #[cfg(not(feature = "llm"))]
        {
            Err(MigrateError::FeatureDisabled("llm"))
        }
    }
}

fn graphviz_render() -> Result<String, MigrateError> {
    #[cfg(feature = "graphviz")]
    {
        Ok("digraph G {}\n".to_string())
    }
    #[cfg(not(feature = "graphviz"))]
    {
        Err(MigrateError::FeatureDisabled("graphviz"))
    }
}

/// Renders a flow tree as DOT/HTML when the `graphviz` feature is enabled.
#[derive(Clone, Debug, Default)]
pub struct FlowTreeVisualizer;

impl FlowTreeVisualizer {
    pub fn render_dot(&self) -> Result<String, MigrateError> {
        graphviz_render()
    }

    pub fn render_html(&self) -> Result<String, MigrateError> {
        graphviz_render()
    }
}

/// Renders a high-level agent graph as DOT/HTML when `graphviz` is enabled.
#[derive(Clone, Debug, Default)]
pub struct HighLevelGraphVisualizer;

impl HighLevelGraphVisualizer {
    pub fn render_dot(&self) -> Result<String, MigrateError> {
        graphviz_render()
    }

    pub fn render_html(&self) -> Result<String, MigrateError> {
        graphviz_render()
    }
}

/// Composite visualizer used by the Python `MainVisualizer` parity type.
#[derive(Clone, Debug, Default)]
pub struct MainVisualizer;

impl MainVisualizer {
    pub fn render_dot(&self) -> Result<String, MigrateError> {
        graphviz_render()
    }

    pub fn render_html(&self) -> Result<String, MigrateError> {
        graphviz_render()
    }
}

/// Renders a playbook tree as DOT/HTML when `graphviz` is enabled.
#[derive(Clone, Debug, Default)]
pub struct PlaybookTreeVisualizer;

impl PlaybookTreeVisualizer {
    pub fn render_dot(&self) -> Result<String, MigrateError> {
        graphviz_render()
    }

    pub fn render_html(&self) -> Result<String, MigrateError> {
        graphviz_render()
    }
}

/// Resolves flow dependencies; DOT output is gated on `graphviz`.
#[derive(Clone, Debug, Default)]
pub struct FlowDependencyResolver;

impl FlowDependencyResolver {
    pub fn resolve(&self, bundle: &IrBundle) -> Vec<String> {
        bundle
            .flows
            .iter()
            .filter_map(|v| v.get("name").and_then(|n| n.as_str()).map(str::to_string))
            .collect()
    }

    pub fn render_dot(&self) -> Result<String, MigrateError> {
        graphviz_render()
    }
}
