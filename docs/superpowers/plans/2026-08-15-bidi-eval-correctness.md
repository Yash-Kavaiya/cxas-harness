# Phase 2 Bidi-Streaming and Eval Correctness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `cxas-evals` with a `TurnCursor` that advances, a `BidiSession` that waits for the agent turn, a dedicated eval quota, and a pluggable `AudioScorer`.

**Architecture:** Explicit `TurnState` machine plus `tokio::select!` in `drive_turn`. Simulations consume `TurnCursor::next()`. Voice turns must produce audio or fail `MissingAgentAudio`. Combined reports include per-turn rows.

**Tech Stack:** Rust 2021, `tokio` (`macros`, `rt-multi-thread`, `time`), `async-trait`, `thiserror`, `serde`/`serde_json`/`serde_yaml`, `cxas-core`, `cxas-proto`, `cxas-state`.

**Spec:** `docs/superpowers/specs/2026-08-15-bidi-eval-correctness-design.md`

## Global Constraints

- Language: Rust 2021 edition, MSRV 1.80.
- Async runtime: `tokio` (full) only in crates that perform I/O; `cxas-parity` is sync.
- gRPC/protobuf: `tonic` + `prost` only; no Python protobuf stubs.
- `location` is never defaulted to `"global"`.
- Feature flags isolate optional integrations (Sheets, BigQuery, TUI, audio).
- Machine-first CLI: structured JSON, stable exit codes, non-interactive by default.
- Issue-driven quality bar: 25 cataloged `GoogleCloudPlatform/cxas-scrapi` issues each require a closing test before release candidate.
- Apache-2.0 license headers on every new Rust file.
- No Gauntlet Loop runtime; Superpowers spec→plan is the development process for this repository.

---

## File map

- Modify: `crates/cxas-evals/Cargo.toml`, `src/lib.rs`
- Create: `crates/cxas-evals/src/error.rs`
- Create: `crates/cxas-evals/src/turn_state.rs`
- Create: `crates/cxas-evals/src/cursor.rs`
- Create: `crates/cxas-evals/src/bidi.rs`
- Create: `crates/cxas-evals/src/simulation.rs`
- Create: `crates/cxas-evals/src/audio.rs`
- Create: `crates/cxas-evals/src/report.rs`
- Create: `crates/cxas-evals/src/tool_evals.rs`, `callback_evals.rs`, `guardrail_evals.rs`, `turn_evals.rs`
- Test: `crates/cxas-evals/tests/cursor.rs`
- Test: `crates/cxas-evals/tests/bidi_dtmf.rs`
- Test: `crates/cxas-evals/tests/audio_score.rs`
- Test: `crates/cxas-evals/tests/report.rs`
- Test: `crates/cxas-evals/tests/quota.rs`

---

### Task 1: `TurnCursor` advances past the first utterance (#355)

**Files:**
- Create: `crates/cxas-evals/src/error.rs`
- Create: `crates/cxas-evals/src/cursor.rs`
- Modify: `crates/cxas-evals/src/lib.rs`
- Test: `crates/cxas-evals/tests/cursor.rs`

**Interfaces:**
- Consumes: nothing
- Produces: `UserInput::{Text,Dtmf,Audio}`, `TurnCursor::{new,next,peek,remaining,is_exhausted}`

- [ ] **Step 1: Write the failing test**

```rust
use cxas_evals::{TurnCursor, UserInput};

#[test]
fn next_advances_instead_of_repeating_the_first_utterance() {
    let mut c = TurnCursor::new(vec![
        UserInput::Text("alpha".into()),
        UserInput::Text("beta".into()),
        UserInput::Text("gamma".into()),
    ]);
    assert_eq!(c.next(), Some(&UserInput::Text("alpha".into())));
    assert_eq!(c.next(), Some(&UserInput::Text("beta".into())));
    assert_eq!(c.next(), Some(&UserInput::Text("gamma".into())));
    assert_eq!(c.next(), None);
    assert!(c.is_exhausted());
}

#[test]
fn peek_does_not_advance() {
    let c = TurnCursor::new(vec![UserInput::Text("only".into())]);
    assert_eq!(c.peek(), Some(&UserInput::Text("only".into())));
    assert_eq!(c.remaining(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-evals next_advances_instead_of_repeating_the_first_utterance --offline`
Expected: FAIL with `cannot find struct TurnCursor`

- [ ] **Step 3: Write minimal implementation**

```rust
pub enum UserInput {
    Text(String),
    Dtmf(String),
    Audio(bytes::Bytes),
}

pub struct TurnCursor {
    utterances: Vec<UserInput>,
    index: usize,
}

impl TurnCursor {
    pub fn new(utterances: Vec<UserInput>) -> Self {
        Self { utterances, index: 0 }
    }
    pub fn next(&mut self) -> Option<&UserInput> {
        let item = self.utterances.get(self.index)?;
        self.index += 1;
        Some(item)
    }
    pub fn peek(&self) -> Option<&UserInput> {
        self.utterances.get(self.index)
    }
    pub fn remaining(&self) -> usize {
        self.utterances.len().saturating_sub(self.index)
    }
    pub fn is_exhausted(&self) -> bool {
        self.index >= self.utterances.len()
    }
}
```

Implement `PartialEq` on `UserInput`. Add `bytes` dep.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-evals --offline`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cxas-evals
git commit -m "feat(evals): advance simulation cursor past first static utterance (#355)"
```

---

### Task 2: `BidiSession::drive_turn` waits for the agent (#345)

**Files:**
- Create: `crates/cxas-evals/src/turn_state.rs`
- Create: `crates/cxas-evals/src/bidi.rs`
- Test: `crates/cxas-evals/tests/bidi_dtmf.rs`

**Interfaces:**
- Consumes: `UserInput`, `TurnState`
- Produces: `TurnState::{AwaitingUserTurn,AwaitingAgentTurn,Terminated}`, `trait CesBidi`, `BidiSession::drive_turn`, `EvalError::AgentTurnTimeout`, `EvalError::IllegalTransition`

- [ ] **Step 1: Write the failing test**

```rust
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
    let mut session = BidiSession::new(
        Scripted { events: vec![] },
        Duration::from_millis(50),
    );
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-evals silent_agent_times_out_instead_of_hanging --offline`
Expected: FAIL with `cannot find struct BidiSession`

- [ ] **Step 3: Write minimal implementation**

`drive_turn`: require `AwaitingUserTurn`; `send_user`; set `AwaitingAgentTurn`; loop `tokio::select!` on `recv_agent()` vs `tokio::time::sleep(deadline)`; on `TurnComplete` set `AwaitingUserTurn` and return; on `SessionEnd` set `Terminated`; on sleep fire `AgentTurnTimeout`. Track `dtmf_acked` when `DtmfAck` is seen. `EvalError` as specified.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-evals --offline`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cxas-evals
git commit -m "feat(evals): wait for agent turn or timeout on bidi DTMF (#345)"
```

---

### Task 3: `SimulationEvals` uses the cursor and eval quota (#355, #263)

**Files:**
- Create: `crates/cxas-evals/src/simulation.rs`
- Test: `crates/cxas-evals/tests/quota.rs`

**Interfaces:**
- Consumes: `TurnCursor`, `BidiSession`, `cxas_core::{ClientConfig, QuotaKind, Sessions}`
- Produces: `SimulationEvals::new`, `run_simulations`, `SimulationPlan`, `SimCase`

- [ ] **Step 1: Write the failing test**

```rust
use cxas_core::{ClientConfig, Credentials, Location, QuotaKind};
use cxas_evals::{SimulationEvals, SimulationPlan, SimCase, UserInput};

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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-evals simulation_sends_each_utterance_once_in_order --offline`
Expected: FAIL with `cannot find struct SimulationEvals`

- [ ] **Step 3: Write minimal implementation**

`SimulationEvals` stores `quota_kind: QuotaKind::EvaluationRunSession` and a factory that builds a `CesBidi`. `run_simulations` for each case creates a `TurnCursor` and a `BidiSession`, then `while let Some(input) = cursor.next() { session.drive_turn(input).await? }`. `RecordingBidi` records `UserInput::Text` / `Dtmf` display strings and immediately yields `TurnComplete`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-evals --offline`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cxas-evals
git commit -m "feat(evals): run simulations with eval quota and advancing cursor (#355, #263)"
```

---

### Task 4: Audio scorer and SpeechPath (#136, #27, #188)

**Files:**
- Create: `crates/cxas-evals/src/audio.rs`
- Modify: `crates/cxas-evals/src/simulation.rs` (call scorer on audio modality)
- Test: `crates/cxas-evals/tests/audio_score.rs`

**Interfaces:**
- Consumes: `CompletedTurn.agent_audio`, `Modality`
- Produces: `trait AudioScorer`, `AudioScore`, `SpeechPathScorer`, `TranscriptExactScorer`, `EvalError::MissingAgentAudio`

- [ ] **Step 1: Write the failing test**

```rust
use cxas_evals::{AudioScorer, EvalError, SpeechPathScorer, TranscriptExactScorer};

#[test]
fn exact_scorer_passes_on_normalized_transcript() {
    let s = TranscriptExactScorer::default();
    let score = s.score(b"ignored", "Hello world").unwrap();
    // TranscriptExactScorer uses the expected string as the "transcript" when
    // the caller sets last_transcript via with_transcript.
    let s = TranscriptExactScorer::with_transcript("hello world");
    let score = s.score(b"xxxx", "Hello world").unwrap();
    assert!(score.passed);
    assert_eq!(score.transcript, "hello world");
}

#[test]
fn missing_audio_is_an_error_on_voice_turns() {
    let err = cxas_evals::require_audio(&[]).unwrap_err();
    assert!(matches!(err, EvalError::MissingAgentAudio));
}

#[tokio::test]
async fn speech_path_uses_injected_stt() {
    struct Fake;
    impl cxas_evals::HttpStt for Fake {
        async fn transcribe(&self, _audio: &[u8]) -> Result<String, EvalError> {
            Ok("hello world".into())
        }
    }
    let scorer = SpeechPathScorer::new(Fake);
    let score = scorer.score(b"\x00\x01", "Hello world").unwrap();
    assert!(score.passed);
    assert!(score.match_score >= 0.8);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-evals missing_audio_is_an_error_on_voice_turns --offline`
Expected: FAIL with `cannot find function require_audio`

- [ ] **Step 3: Write minimal implementation**

`require_audio(bytes)` returns `Err(MissingAgentAudio)` if `bytes.is_empty()`, else `Ok`. `TranscriptExactScorer` compares normalized (lowercase, collapsed whitespace) expected vs configured transcript. `SpeechPathScorer<H: HttpStt>` calls `transcribe`, then the same normalize+threshold 0.8. `SimulationEvals` for `Modality::Audio` calls `require_audio` then `scorer.score`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-evals --offline`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cxas-evals
git commit -m "feat(evals): score returned audio via SpeechPath (#136, #27, #188)"
```

---

### Task 5: Combined report turn rows and remaining eval types (#206)

**Files:**
- Create: `crates/cxas-evals/src/report.rs`
- Create: `crates/cxas-evals/src/tool_evals.rs`
- Create: `crates/cxas-evals/src/callback_evals.rs`
- Create: `crates/cxas-evals/src/guardrail_evals.rs`
- Create: `crates/cxas-evals/src/turn_evals.rs`
- Test: `crates/cxas-evals/tests/report.rs`

**Interfaces:**
- Consumes: `CompletedTurn`, `AudioScore`
- Produces: `EvalReport`, `TurnRow`, `generate_combined_json_report`, types `ToolEvals`, `CallbackEvals`, `GuardrailEvals`, `TurnEvals`

- [ ] **Step 1: Write the failing test**

```rust
use cxas_evals::{generate_combined_json_report, EvalReport, TurnRow};

#[test]
fn json_report_includes_turn_rows() {
    let report = EvalReport {
        summary: cxas_evals::ReportSummary {
            passed: 1,
            failed: 0,
            errored: 0,
        },
        turns: vec![
            TurnRow {
                case_id: "c1".into(),
                turn_index: 0,
                user: "hi".into(),
                agent_text: "hello".into(),
                audio: None,
                expectation_results: vec![],
                latency_ms: 3,
            },
            TurnRow {
                case_id: "c1".into(),
                turn_index: 1,
                user: "bye".into(),
                agent_text: "goodbye".into(),
                audio: None,
                expectation_results: vec![],
                latency_ms: 4,
            },
        ],
    };
    let json = generate_combined_json_report(&report);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["turns"].as_array().unwrap().len(), 2);
    assert_eq!(v["turns"][1]["turn_index"], 1);
}

#[test]
fn parity_eval_types_exist() {
    assert_eq!(cxas_evals::ToolEvals::crate_label(), "ToolEvals");
    assert_eq!(cxas_evals::CallbackEvals::crate_label(), "CallbackEvals");
    assert_eq!(cxas_evals::GuardrailEvals::crate_label(), "GuardrailEvals");
    assert_eq!(cxas_evals::TurnEvals::crate_label(), "TurnEvals");
    assert_eq!(cxas_evals::SimulationEvals::crate_label(), "SimulationEvals");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cxas-evals json_report_includes_turn_rows --offline`
Expected: FAIL with `cannot find function generate_combined_json_report`

- [ ] **Step 3: Write minimal implementation**

Serialize `EvalReport` with serde. Each eval type is a struct with `pub fn crate_label() -> &'static str`. `SimulationEvals::run_simulations` already fills `turns` from `drive_turn` results (add that mapping if missing: `turn_index` from the cursor index minus one).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p cxas-evals --offline`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cxas-evals
git commit -m "feat(evals): include per-turn rows in combined JSON report (#206)"
```
