# Phase 2 — Bidi-Streaming and Eval Correctness Design

**Date:** 2026-08-15
**Status:** Approved from the product briefs (retired 2026-08-16; requirements restated here)
**Product:** `cxas-harness`
**Phase:** 2 of 5 — bidi-streaming and concurrency correctness
**Depends on:** Phase 0 parity contract, Phase 1 workspace / `cxas-core` / `Location` / `QuotaKind`

## Purpose

Replace the Python eval loop's implicit flags with an explicit turn-state machine, a `tokio::select!` bidi session handler, a dedicated evaluation-quota session client, and a pluggable audio scorer. This phase ships a working `cxas-evals` crate that can drive simulated conversations against a `CesTransport` mock and score text or audio turns.

**Issue-driven quality bar:** this phase closes **#355** (simulation repeats the first `static_utterance`), **#345** (DTMF simulations hang because `BidiSessionHandler` never waits for the agent turn), **#136** (voice simulations ignore returned audio), **#27** (Audio Evaluations), **#188** (SpeechPath), **#206** (turn-eval rows in the combined report), and finishes **#263** (eval `RunSession` quota is the only quota the eval client uses). `cxas-scrapi` parity means `SimulationEvals`, `TurnEvals`, `ToolEvals`, `CallbackEvals`, and `GuardrailEvals` from the Phase 0 manifest exist as Rust types with the same verbs.

## Architecture

`cxas-evals` owns the simulation cursor, the bidi turn machine, scorers, and report assembly. It talks to CES only through `cxas_core::Sessions` / `cxas_core::Evaluations` (already location-mandatory, quota-typed).

```
SimulationPlan (utterances[], dtmf[], expectations[])
        |
        v
TurnCursor  ----next()---->  next user input (never repeats unless plan says so)
        |
        v
BidiSession  (TurnState + tokio::select!)
        |
        +--> agent text  --> ExpectationScorer
        +--> agent audio --> AudioScorer (SpeechPath)
        |
        v
EvalReport (summary + per-turn rows including turn-eval metrics)
```

The Python bug class is two missing pieces: (1) a cursor that advances, (2) a state enum that distinguishes "we are waiting for the user" from "we are waiting for the agent". Both are types, not booleans.

## Components

### 1. `TurnState`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnState {
    AwaitingUserTurn,
    AwaitingAgentTurn,
    Terminated,
}
```

Illegal transitions (`AwaitingUserTurn` --agent_chunk--> `AwaitingUserTurn`, `Terminated` --anything--> not `Terminated`) return `EvalError::IllegalTransition { from, to, event }`.

### 2. `TurnCursor` (#355)

```rust
pub struct TurnCursor {
    utterances: Vec<UserInput>,
    index: usize,
}

pub enum UserInput {
    Text(String),
    Dtmf(String),
    Audio(Bytes),
}

impl TurnCursor {
    pub fn new(utterances: Vec<UserInput>) -> Self;
    pub fn next(&mut self) -> Option<&UserInput>; // advances index
    pub fn peek(&self) -> Option<&UserInput>;     // does not advance
    pub fn remaining(&self) -> usize;
    pub fn is_exhausted(&self) -> bool;
}
```

`next` increments `index` **before** returning the subsequent call's value. The first call returns `utterances[0]`; the second returns `utterances[1]`. A regression test feeds `["hi", "help", "bye"]` and asserts the session transport received those three strings in order — not `["hi", "hi", "hi"]`.

### 3. `BidiSession` (#345)

```rust
pub struct BidiSession<T: CesBidi> {
    state: TurnState,
    transport: T,
    deadline: Duration,
}

pub trait CesBidi {
    fn send_user(&mut self, input: &UserInput) -> impl Future<Output = Result<(), EvalError>>;
    fn recv_agent(&mut self) -> impl Future<Output = Result<AgentEvent, EvalError>>;
    fn close(&mut self) -> impl Future<Output = Result<(), EvalError>>;
}

pub enum AgentEvent {
    TextDelta(String),
    AudioDelta(Bytes),
    DtmfAck,
    TurnComplete,
    SessionEnd,
}

impl<T: CesBidi> BidiSession<T> {
    pub async fn drive_turn(&mut self, input: &UserInput) -> Result<CompletedTurn, EvalError>;
}
```

`drive_turn` algorithm (normative):

1. Require `state == AwaitingUserTurn`; else `IllegalTransition`.
2. `send_user(input)`.
3. Set `state = AwaitingAgentTurn`.
4. Loop on `tokio::select!`:
   - `recv_agent()` → accumulate text/audio; on `TurnComplete` break; on `SessionEnd` set `Terminated` and break.
   - `_ = sleep(deadline)` → `EvalError::AgentTurnTimeout` (this is the #345 hang fix: the handler **always** waits for an agent event or times out; it never returns to the caller while still in `AwaitingAgentTurn` without an error).
5. If still running, set `state = AwaitingUserTurn`.
6. Return `CompletedTurn { user, agent_text, agent_audio, dtmf_acked }`.

DTMF inputs use the same path. The Python hang was "send DTMF, do not wait for agent, immediately send the next DTMF". The Rust path cannot send a second user input until `drive_turn` returns.

### 4. `SimulationEvals`

```rust
pub struct SimulationEvals {
    sessions: Sessions,          // QuotaKind::EvaluationRunSession (#263)
    location: Location,          // required, from ClientConfig (#401 inherited)
    scorer: Box<dyn AudioScorer>,
}

impl SimulationEvals {
    pub fn new(config: ClientConfig, scorer: Box<dyn AudioScorer>) -> Self;
    pub async fn run_simulations(&self, plan: SimulationPlan) -> Result<EvalReport, EvalError>;
}
```

`SimulationPlan` holds `cases: Vec<SimCase>`, each with `id`, `utterances: Vec<UserInput>`, `expectations: Vec<Expectation>`, `modality: Modality` (`Text` | `Audio` | `Dtmf`).

`run_simulations` for each case: create session → `TurnCursor` → while `next()` is `Some`, `drive_turn` → score → push a turn row. After the cursor is exhausted, mark the case complete. It never rewinds the cursor.

### 5. `AudioScorer` and SpeechPath (#136, #27, #188)

```rust
pub trait AudioScorer: Send + Sync {
    fn score(&self, audio: &[u8], expected_transcript: &str) -> Result<AudioScore, EvalError>;
}

pub struct AudioScore {
    pub transcript: String,
    pub match_score: f32, // 0.0..=1.0
    pub passed: bool,
}

pub struct SpeechPathScorer { /* STT adapter; default uses a traity HttpStt */ }
pub struct TranscriptExactScorer; // test double: compares provided transcript
```

Voice-channel simulations **must** call `scorer.score` on `CompletedTurn.agent_audio` when `modality == Audio`. If `agent_audio` is empty, the turn fails with `EvalError::MissingAgentAudio` (#136). Text-only cases do not require audio.

SpeechPath (#188) is the production `AudioScorer` implementation: it sends PCM/WAV to a configured STT endpoint (Gemini / Cloud Speech, injected as `HttpStt`) and compares the transcript to the expected string using normalized whitespace and case-folding. Threshold default: `match_score >= 0.8` passes.

### 6. Combined report turn rows (#206)

```rust
pub struct EvalReport {
    pub summary: ReportSummary,
    pub turns: Vec<TurnRow>,
}

pub struct TurnRow {
    pub case_id: String,
    pub turn_index: usize,
    pub user: String,
    pub agent_text: String,
    pub audio: Option<AudioScore>,
    pub expectation_results: Vec<ExpectationResult>,
    pub latency_ms: u64,
}
```

`generate_combined_html_report` / `generate_combined_json_report` include every `TurnRow`. JSON is the machine-readable form Phase 5's CLI emits.

### 7. Other eval types (parity)

`ToolEvals::run_tool_tests`, `CallbackEvals::test_all_callbacks_in_app_dir`, `GuardrailEvals::run`, `TurnEvals::run` exist as async methods taking fixture paths + `ClientConfig`. Their scoring is expectation-based (string/JSON equality) and does not use the bidi machine unless the fixture sets `modality: audio`.

## Data flow

**Text simulation**

1. Load `SimulationPlan` from YAML (`utterances: [hi, help, bye]`).
2. `TurnCursor::new` → index 0.
3. `BidiSession` starts in `AwaitingUserTurn`.
4. `next()` yields `"hi"`; `drive_turn` sends it, waits for `TurnComplete`, returns to `AwaitingUserTurn`.
5. Repeat for `"help"` and `"bye"`.
6. Cursor exhausted → report.

**DTMF simulation (#345)**

Same as text, but `UserInput::Dtmf("1")`. After send, the select loop **must** observe `DtmfAck` or `TurnComplete` or timeout. A mock that never yields an agent event produces `AgentTurnTimeout` within `deadline` (default 10s, injectable).

**Voice simulation (#136 / #27 / #188)**

1. Modality `Audio`.
2. Each `CompletedTurn.agent_audio` is passed to `AudioScorer`.
3. `TurnRow.audio` is `Some`; combined report includes transcript + score.

**Quota (#263)**

Every session opened by `SimulationEvals` is constructed with `QuotaKind::EvaluationRunSession`. A unit test on the mock transport asserts the resource name or header cannot be the general `RunSession` quota.

## Error handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    #[error("illegal turn transition {from:?} --{event}--> {to:?}")]
    IllegalTransition { from: TurnState, to: TurnState, event: &'static str },
    #[error("agent did not complete its turn within {0:?}")]
    AgentTurnTimeout(Duration),
    #[error("simulation cursor is exhausted")]
    CursorExhausted,
    #[error("voice turn produced no agent audio")]
    MissingAgentAudio,
    #[error("audio score failed: {0}")]
    AudioScore(String),
    #[error(transparent)]
    Core(#[from] cxas_core::CoreError),
}
```

| Condition | Behavior |
|---|---|
| Cursor `next()` after last utterance | `None`; runner stops the case (does not wrap) |
| `drive_turn` while `Terminated` | `IllegalTransition` |
| Agent silent past deadline | `AgentTurnTimeout` — **not** a hang |
| Audio modality, empty bytes | `MissingAgentAudio` — fail the turn, continue the case |
| STT HTTP 4xx/5xx | `AudioScore` with status; turn fails |
| Core `LocationRequired` | propagated; evals cannot invent `"global"` |

`run_simulations` collects per-case errors into `EvalReport` rather than aborting the whole batch, except for `LocationRequired` and transport auth failures which abort the batch.

## Testing

Every test uses a mock `CesBidi` / `CesTransport`. No live CES.

1. **#355** — plan `["alpha", "beta", "gamma"]`; mock records sent inputs; assert `== ["alpha", "beta", "gamma"]`. A second assertion: `cursor.next()` after construction is `"alpha"` and a subsequent `next()` is `"beta"`.
2. **#345** — DTMF plan of two digits; mock yields `DtmfAck` then `TurnComplete` for each; both digits are sent. A hang-regression test: mock that never yields; `drive_turn` with `deadline = 50ms` returns `AgentTurnTimeout` and does so in < 500ms.
3. **#136 / #27** — audio modality; mock yields `AudioDelta` + `TurnComplete`; `TranscriptExactScorer` (test double given the expected transcript via the event) marks `passed`. A sibling test with empty audio asserts `MissingAgentAudio`.
4. **#188** — `SpeechPathScorer` with a mock `HttpStt` that returns `"hello world"`; expected `"Hello world"` passes after normalize.
5. **#206** — report JSON contains `turns` array with `turn_index` 0..n-1 for a 3-turn case.
6. **#263** — mock `Sessions` constructor captured `QuotaKind::EvaluationRunSession`.
7. **Illegal transition** — force `state = Terminated`; `drive_turn` errs.
8. **Parity hook** — `cxas-evals` types named in the Phase 0 manifest exist (`SimulationEvals`, `TurnEvals`, `ToolEvals`, `CallbackEvals`, `GuardrailEvals`).
9. **Latency** — `TurnRow.latency_ms` is ≥ 0 and is the elapsed time inside `drive_turn`.

## Out of scope

- `cxas lint` / `llm-lint` (Phase 3).
- Hillclimbing snapshot cleanup (Phase 4).
- CLI flags `--modality audio`, `evals report` wiring (Phase 5 consumes `EvalReport`).
- Real Cloud Speech / Gemini network calls in this phase's tests.
