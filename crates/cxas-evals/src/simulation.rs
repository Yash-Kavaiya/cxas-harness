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

use crate::{
    AgentEvent, AudioScorer, BidiSession, CesBidi, EvalError, EvalReport, ExpectationResult,
    ReportSummary, TurnCursor, TurnRow, UserInput,
};
use cxas_core::{ClientConfig, CoreError, QuotaKind};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const DEFAULT_TURN_DEADLINE: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modality {
    Text,
    Audio,
    Dtmf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expectation {
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct SimCase {
    pub id: String,
    pub utterances: Vec<UserInput>,
    pub expectations: Vec<Expectation>,
    pub modality: Modality,
}

#[derive(Debug, Clone)]
pub struct SimulationPlan {
    pub cases: Vec<SimCase>,
}

type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

trait ErasedCesBidi: Send {
    fn send_user<'a>(&'a mut self, input: &'a UserInput) -> BoxFut<'a, Result<(), EvalError>>;
    fn recv_agent(&mut self) -> BoxFut<'_, Result<AgentEvent, EvalError>>;
    fn close(&mut self) -> BoxFut<'_, Result<(), EvalError>>;
}

impl<T: CesBidi + Send> ErasedCesBidi for T {
    fn send_user<'a>(&'a mut self, input: &'a UserInput) -> BoxFut<'a, Result<(), EvalError>> {
        Box::pin(CesBidi::send_user(self, input))
    }

    fn recv_agent(&mut self) -> BoxFut<'_, Result<AgentEvent, EvalError>> {
        Box::pin(CesBidi::recv_agent(self))
    }

    fn close(&mut self) -> BoxFut<'_, Result<(), EvalError>> {
        Box::pin(CesBidi::close(self))
    }
}

struct BoxedBidi {
    inner: Box<dyn ErasedCesBidi>,
}

impl BoxedBidi {
    fn new<T: CesBidi + Send + 'static>(inner: T) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }
}

impl CesBidi for BoxedBidi {
    async fn send_user(&mut self, input: &UserInput) -> Result<(), EvalError> {
        self.inner.send_user(input).await
    }

    async fn recv_agent(&mut self) -> Result<AgentEvent, EvalError> {
        self.inner.recv_agent().await
    }

    async fn close(&mut self) -> Result<(), EvalError> {
        self.inner.close().await
    }
}

pub struct RecordingBidi {
    sent: Arc<Mutex<Vec<String>>>,
    pending_complete: bool,
}

impl RecordingBidi {
    pub fn new(sent: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            sent,
            pending_complete: false,
        }
    }
}

impl CesBidi for RecordingBidi {
    async fn send_user(&mut self, input: &UserInput) -> Result<(), EvalError> {
        self.sent.lock().expect("lock").push(input.display_text());
        self.pending_complete = true;
        Ok(())
    }

    async fn recv_agent(&mut self) -> Result<AgentEvent, EvalError> {
        if self.pending_complete {
            self.pending_complete = false;
            Ok(AgentEvent::TurnComplete)
        } else {
            std::future::pending().await
        }
    }

    async fn close(&mut self) -> Result<(), EvalError> {
        Ok(())
    }
}

pub struct SimulationEvals {
    #[allow(dead_code)]
    config: ClientConfig,
    scorer: Box<dyn AudioScorer>,
    factory: Box<dyn Fn() -> BoxedBidi + Send + Sync>,
    quota_kind: QuotaKind,
}

impl SimulationEvals {
    pub fn crate_label() -> &'static str {
        "SimulationEvals"
    }

    pub fn new(config: ClientConfig, scorer: Box<dyn AudioScorer>) -> Self {
        Self::new_with_factory(config, scorer, || {
            RecordingBidi::new(Arc::new(Mutex::new(Vec::new())))
        })
    }

    pub fn new_with_factory<F, T>(
        config: ClientConfig,
        scorer: Box<dyn AudioScorer>,
        factory: F,
    ) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
        T: CesBidi + Send + 'static,
    {
        Self {
            config,
            scorer,
            factory: Box::new(move || BoxedBidi::new(factory())),
            quota_kind: QuotaKind::EvaluationRunSession,
        }
    }

    pub fn quota_kind(&self) -> QuotaKind {
        self.quota_kind
    }

    pub async fn run_simulations(&self, plan: SimulationPlan) -> Result<EvalReport, EvalError> {
        let mut summary = ReportSummary::default();
        let mut turns = Vec::new();

        for case in plan.cases {
            let mut cursor = TurnCursor::new(case.utterances);
            let mut session = BidiSession::new((self.factory)(), DEFAULT_TURN_DEADLINE);
            let mut case_failed = false;

            while let Some(input) = cursor.next().cloned() {
                let user = input.display_text();
                let turn_index = cursor.index().saturating_sub(1);
                match session.drive_turn(&input).await {
                    Ok(completed) => {
                        let mut audio = None;
                        if case.modality == Modality::Audio {
                            match crate::require_audio(&completed.agent_audio) {
                                Ok(bytes) => {
                                    let expected = case
                                        .expectations
                                        .get(turn_index)
                                        .map(|e| e.text.as_str())
                                        .unwrap_or("");
                                    match self.scorer.score(bytes, expected) {
                                        Ok(score) => {
                                            if !score.passed {
                                                case_failed = true;
                                            }
                                            audio = Some(score);
                                        }
                                        Err(EvalError::Core(CoreError::LocationRequired)) => {
                                            return Err(EvalError::Core(
                                                CoreError::LocationRequired,
                                            ));
                                        }
                                        Err(_) => {
                                            case_failed = true;
                                            summary.errored += 1;
                                        }
                                    }
                                }
                                Err(EvalError::MissingAgentAudio) => {
                                    case_failed = true;
                                    summary.failed += 1;
                                }
                                Err(EvalError::Core(CoreError::LocationRequired)) => {
                                    return Err(EvalError::Core(CoreError::LocationRequired));
                                }
                                Err(_) => {
                                    case_failed = true;
                                    summary.errored += 1;
                                }
                            }
                        }

                        let expectation_results = case
                            .expectations
                            .iter()
                            .map(|exp| {
                                let passed = completed.agent_text == exp.text;
                                if !passed {
                                    case_failed = true;
                                }
                                ExpectationResult {
                                    expected: exp.text.clone(),
                                    actual: completed.agent_text.clone(),
                                    passed,
                                }
                            })
                            .collect();

                        turns.push(TurnRow {
                            case_id: case.id.clone(),
                            turn_index,
                            user,
                            agent_text: completed.agent_text,
                            audio,
                            expectation_results,
                            latency_ms: completed.latency_ms,
                        });
                    }
                    Err(EvalError::Core(CoreError::LocationRequired)) => {
                        return Err(EvalError::Core(CoreError::LocationRequired));
                    }
                    Err(EvalError::Core(CoreError::Transport(msg)))
                        if msg.to_ascii_lowercase().contains("auth") =>
                    {
                        return Err(EvalError::Core(CoreError::Transport(msg)));
                    }
                    Err(_) => {
                        summary.errored += 1;
                        case_failed = true;
                    }
                }
            }

            if !case_failed {
                summary.passed += 1;
            } else if summary.errored == 0 {
                summary.failed += 1;
            }
        }

        Ok(EvalReport { summary, turns })
    }
}
