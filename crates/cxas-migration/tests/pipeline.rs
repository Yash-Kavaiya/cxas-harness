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

use cxas_core::Location;
use cxas_migration::{
    ConversationalAgentsAPI, DFCXAgentExporter, DFCXConversationRunner, MigrationPipeline,
    MigrationSource, MigrationTarget, Profile,
};

#[test]
fn default_pipeline_is_non_interactive() {
    let p = MigrationPipeline::default();
    assert!(p.yes);
    assert_eq!(p.profile, Profile::Standard);
}

#[tokio::test]
async fn run_without_display_name_is_usage_error() {
    let p = MigrationPipeline::default();
    let err = p
        .run(
            MigrationSource::Zip(std::path::PathBuf::from("agent.zip")),
            MigrationTarget {
                project_id: "p".into(),
                location: Location::new("us").unwrap(),
                display_name: String::new(),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, cxas_migration::MigrateError::Usage(_)));
}

#[test]
fn location_new_rejects_blank_before_export() {
    assert!(Location::new("").is_err());
}

#[test]
fn parity_types_are_exported() {
    let _ = std::any::type_name::<DFCXAgentExporter>();
    let _ = std::any::type_name::<ConversationalAgentsAPI>();
    let _ = std::any::type_name::<DFCXConversationRunner>();
    let _ = std::any::type_name::<cxas_migration::FlowTreeVisualizer>();
    let _ = std::any::type_name::<cxas_migration::HighLevelGraphVisualizer>();
    let _ = std::any::type_name::<cxas_migration::MainVisualizer>();
    let _ = std::any::type_name::<cxas_migration::PlaybookTreeVisualizer>();
    let _ = std::any::type_name::<cxas_migration::FlowDependencyResolver>();
}
