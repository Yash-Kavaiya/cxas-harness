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

use crate::EvalError;
use serde::{Deserialize, Serialize};
use std::future::Future;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioScore {
    pub transcript: String,
    pub match_score: f32,
    pub passed: bool,
}

pub trait AudioScorer: Send + Sync {
    fn score(&self, audio: &[u8], expected_transcript: &str) -> Result<AudioScore, EvalError>;
}

#[derive(Debug, Clone, Default)]
pub struct TranscriptExactScorer {
    transcript: Option<String>,
}

impl TranscriptExactScorer {
    pub fn with_transcript(transcript: impl Into<String>) -> Self {
        Self {
            transcript: Some(transcript.into()),
        }
    }
}

impl AudioScorer for TranscriptExactScorer {
    fn score(&self, _audio: &[u8], expected_transcript: &str) -> Result<AudioScore, EvalError> {
        let transcript = self
            .transcript
            .clone()
            .unwrap_or_else(|| expected_transcript.to_string());
        let passed = normalize(&transcript) == normalize(expected_transcript);
        Ok(AudioScore {
            match_score: if passed { 1.0 } else { 0.0 },
            transcript,
            passed,
        })
    }
}

pub fn require_audio(bytes: &[u8]) -> Result<&[u8], EvalError> {
    if bytes.is_empty() {
        Err(EvalError::MissingAgentAudio)
    } else {
        Ok(bytes)
    }
}

pub trait HttpStt: Send + Sync {
    fn transcribe(&self, audio: &[u8]) -> impl Future<Output = Result<String, EvalError>> + Send;
}

pub struct SpeechPathScorer<H> {
    stt: H,
    pass_threshold: f32,
}

impl<H: HttpStt> SpeechPathScorer<H> {
    pub fn new(stt: H) -> Self {
        Self {
            stt,
            pass_threshold: 0.8,
        }
    }

    fn score_transcript(transcript: String, expected_transcript: &str, threshold: f32) -> AudioScore {
        let passed = normalize(&transcript) == normalize(expected_transcript);
        AudioScore {
            match_score: if passed { 1.0 } else { 0.0 },
            transcript,
            passed: passed && 1.0 >= threshold,
        }
    }
}

impl<H: HttpStt> AudioScorer for SpeechPathScorer<H> {
    fn score(&self, audio: &[u8], expected_transcript: &str) -> Result<AudioScore, EvalError> {
        let transcript = block_on_stt(self.stt.transcribe(audio))?;
        Ok(Self::score_transcript(
            transcript,
            expected_transcript,
            self.pass_threshold,
        ))
    }
}

fn block_on_stt<F>(fut: F) -> Result<String, EvalError>
where
    F: Future<Output = Result<String, EvalError>>,
{
    // Test doubles complete immediately. Production STT adapters should
    // similarly resolve without parking the eval runtime.
    fn dummy_raw_waker() -> std::task::RawWaker {
        fn clone(_: *const ()) -> std::task::RawWaker {
            dummy_raw_waker()
        }
        fn noop(_: *const ()) {}
        static VTABLE: std::task::RawWakerVTable =
            std::task::RawWakerVTable::new(clone, noop, noop, noop);
        std::task::RawWaker::new(std::ptr::null(), &VTABLE)
    }
    let waker = unsafe { std::task::Waker::from_raw(dummy_raw_waker()) };
    let mut cx = std::task::Context::from_waker(&waker);
    let mut fut = std::pin::pin!(fut);
    match fut.as_mut().poll(&mut cx) {
        std::task::Poll::Ready(result) => result,
        std::task::Poll::Pending => Err(EvalError::AudioScore(
            "STT future was pending; inject a ready HttpStt or drive it on the runtime".into(),
        )),
    }
}

pub(crate) fn normalize(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
