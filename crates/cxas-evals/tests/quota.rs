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

use cxas_core::{ClientConfig, Credentials, Location, QuotaKind};
use cxas_evals::{SimCase, SimulationEvals, SimulationPlan, UserInput};

#[tokio::test]
async fn simulation_sends_each_utterance_once_in_order() {
    let sent = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let plan = SimulationPlan {
        cases: vec![SimCase {
            id: "c1".into(),
            utterances: vec![
                UserInput::Text("alpha".into()),
                UserInput::Text("beta".into()),
                UserInput::Text("gamma".into()),
            ],
            expectations: vec![],
            modality: cxas_evals::Modality::Text,
        }],
    };
    let ev = SimulationEvals::new_with_factory(
        ClientConfig {
            project_id: "p".into(),
            location: Location::new("us").unwrap(),
            credentials: Credentials::ApplicationDefault,
        },
        Box::new(cxas_evals::TranscriptExactScorer::default()),
        {
            let sent = sent.clone();
            move || cxas_evals::RecordingBidi::new(sent.clone())
        },
    );
    ev.run_simulations(plan).await.unwrap();
    assert_eq!(
        *sent.lock().unwrap(),
        vec!["alpha".to_string(), "beta".into(), "gamma".into()]
    );
    assert_eq!(ev.quota_kind(), QuotaKind::EvaluationRunSession);
}
