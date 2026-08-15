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

use cxas_evals::{AudioScorer, EvalError, SpeechPathScorer, TranscriptExactScorer};

#[test]
fn exact_scorer_passes_on_normalized_transcript() {
    let s = TranscriptExactScorer::default();
    let _score = s.score(b"ignored", "Hello world").unwrap();
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
