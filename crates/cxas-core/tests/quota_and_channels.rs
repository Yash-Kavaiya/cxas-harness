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

use cxas_core::{
    ChannelSettings, ClientConfig, Credentials, DeploymentName, Deployments, Evaluations, Location,
    QuotaKind,
};
use std::sync::Arc;

#[test]
fn evaluations_default_to_evaluation_run_session_quota() {
    let cfg = ClientConfig {
        project_id: "p".into(),
        location: Location::new("us").unwrap(),
        credentials: Credentials::ApplicationDefault,
    };
    let ev = Evaluations::new(cfg, Arc::new(cxas_core::NoopTransport));
    assert_eq!(ev.quota_kind(), QuotaKind::EvaluationRunSession);
}

#[tokio::test]
async fn update_channel_settings_sends_noise_cancellation() {
    let transport = Arc::new(cxas_core::RecordingTransport::default());
    let cfg = ClientConfig {
        project_id: "p".into(),
        location: Location::new("us").unwrap(),
        credentials: Credentials::ApplicationDefault,
    };
    let deps = Deployments::new(cfg, transport.clone());
    deps.update_channel_settings(
        &DeploymentName::parse("projects/p/locations/us/apps/a/deployments/d").unwrap(),
        ChannelSettings {
            noise_cancellation: Some(true),
            noise_suppression_level: Some(2),
        },
    )
    .await
    .unwrap();
    let rec = transport.last_channel_settings().unwrap();
    assert_eq!(rec.noise_cancellation, Some(true));
    assert_eq!(rec.noise_suppression_level, Some(2));
}

#[test]
fn parity_table_covers_core_types() {
    let names = cxas_core::parity_table::CORE_PYTHON_CLASSES;
    assert!(names.contains(&"Apps"));
    assert!(names.contains(&"Evaluations"));
    assert!(names.contains(&"Deployments"));
    let manifest = cxas_parity::load_bundled().unwrap();
    for class in names {
        manifest.require_type(class).unwrap();
    }
}
