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

use cxas_evals::{AgentEvent, BidiSession, CesBidi, EvalError, TurnState, UserInput};
use std::time::Duration;

struct Scripted {
    events: Vec<AgentEvent>,
}

impl CesBidi for Scripted {
    async fn send_user(&mut self, _input: &UserInput) -> Result<(), EvalError> {
        Ok(())
    }
    async fn recv_agent(&mut self) -> Result<AgentEvent, EvalError> {
        if self.events.is_empty() {
            std::future::pending().await
        } else {
            Ok(self.events.remove(0))
        }
    }
    async fn close(&mut self) -> Result<(), EvalError> {
        Ok(())
    }
}

#[tokio::test]
async fn dtmf_turn_waits_for_agent_before_returning() {
    let mut session = BidiSession::new(
        Scripted {
            events: vec![AgentEvent::DtmfAck, AgentEvent::TurnComplete],
        },
        Duration::from_secs(1),
    );
    let turn = session
        .drive_turn(&UserInput::Dtmf("1".into()))
        .await
        .unwrap();
    assert!(turn.dtmf_acked);
    assert_eq!(session.state(), TurnState::AwaitingUserTurn);
}

#[tokio::test]
async fn silent_agent_times_out_instead_of_hanging() {
    let mut session = BidiSession::new(Scripted { events: vec![] }, Duration::from_millis(50));
    let started = std::time::Instant::now();
    let err = session
        .drive_turn(&UserInput::Dtmf("2".into()))
        .await
        .unwrap_err();
    assert!(started.elapsed() < Duration::from_millis(500));
    assert!(matches!(err, EvalError::AgentTurnTimeout(_)));
}

#[tokio::test]
async fn terminated_session_rejects_new_user_input() {
    let mut session = BidiSession::new(
        Scripted {
            events: vec![AgentEvent::SessionEnd],
        },
        Duration::from_secs(1),
    );
    let _ = session
        .drive_turn(&UserInput::Text("hi".into()))
        .await
        .unwrap();
    let err = session
        .drive_turn(&UserInput::Text("again".into()))
        .await
        .unwrap_err();
    assert!(matches!(err, EvalError::IllegalTransition { .. }));
}
