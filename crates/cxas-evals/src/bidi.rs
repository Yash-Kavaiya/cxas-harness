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

use crate::{EvalError, TurnState, UserInput};
use bytes::Bytes;
use std::future::Future;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentEvent {
    TextDelta(String),
    AudioDelta(Bytes),
    DtmfAck,
    TurnComplete,
    SessionEnd,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedTurn {
    pub user: UserInput,
    pub agent_text: String,
    pub agent_audio: Bytes,
    pub dtmf_acked: bool,
    pub latency_ms: u64,
}

pub trait CesBidi {
    fn send_user(
        &mut self,
        input: &UserInput,
    ) -> impl Future<Output = Result<(), EvalError>> + Send;
    fn recv_agent(&mut self) -> impl Future<Output = Result<AgentEvent, EvalError>> + Send;
    fn close(&mut self) -> impl Future<Output = Result<(), EvalError>> + Send;
}

pub struct BidiSession<T: CesBidi> {
    state: TurnState,
    transport: T,
    deadline: Duration,
}

impl<T: CesBidi> BidiSession<T> {
    pub fn new(transport: T, deadline: Duration) -> Self {
        Self {
            state: TurnState::AwaitingUserTurn,
            transport,
            deadline,
        }
    }

    pub fn state(&self) -> TurnState {
        self.state
    }

    pub async fn drive_turn(&mut self, input: &UserInput) -> Result<CompletedTurn, EvalError> {
        if self.state != TurnState::AwaitingUserTurn {
            return Err(EvalError::IllegalTransition {
                from: self.state,
                to: TurnState::AwaitingAgentTurn,
                event: "user_input",
            });
        }

        let started = Instant::now();
        self.transport.send_user(input).await?;
        self.state = TurnState::AwaitingAgentTurn;

        let mut agent_text = String::new();
        let mut agent_audio = Vec::new();
        let mut dtmf_acked = false;
        let mut terminated = false;

        let timeout = tokio::time::sleep(self.deadline);
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                event = self.transport.recv_agent() => {
                    match event? {
                        AgentEvent::TextDelta(delta) => agent_text.push_str(&delta),
                        AgentEvent::AudioDelta(delta) => agent_audio.extend_from_slice(&delta),
                        AgentEvent::DtmfAck => dtmf_acked = true,
                        AgentEvent::TurnComplete => break,
                        AgentEvent::SessionEnd => {
                            terminated = true;
                            break;
                        }
                    }
                }
                _ = &mut timeout => {
                    return Err(EvalError::AgentTurnTimeout(self.deadline));
                }
            }
        }

        if terminated {
            self.state = TurnState::Terminated;
        } else {
            self.state = TurnState::AwaitingUserTurn;
        }

        Ok(CompletedTurn {
            user: input.clone(),
            agent_text,
            agent_audio: Bytes::from(agent_audio),
            dtmf_acked,
            latency_ms: started.elapsed().as_millis() as u64,
        })
    }
}
