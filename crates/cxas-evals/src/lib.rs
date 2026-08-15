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

mod audio;
mod bidi;
mod callback_evals;
mod cursor;
mod error;
mod fixture;
mod guardrail_evals;
mod report;
mod simulation;
mod tool_evals;
mod turn_evals;
mod turn_state;

pub use audio::{
    require_audio, AudioScore, AudioScorer, HttpStt, SpeechPathScorer, TranscriptExactScorer,
};
pub use bidi::{AgentEvent, BidiSession, CesBidi, CompletedTurn};
pub use callback_evals::CallbackEvals;
pub use cursor::{TurnCursor, UserInput};
pub use error::EvalError;
pub use guardrail_evals::GuardrailEvals;
pub use report::{
    generate_combined_html_report, generate_combined_json_report, EvalReport, ExpectationResult,
    ReportSummary, TurnRow,
};
pub use simulation::{
    Expectation, Modality, RecordingBidi, SimCase, SimulationEvals, SimulationPlan,
};
pub use tool_evals::ToolEvals;
pub use turn_evals::TurnEvals;
pub use turn_state::TurnState;

pub fn crate_name() -> &'static str {
    "cxas-evals"
}
