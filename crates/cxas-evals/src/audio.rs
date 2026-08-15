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

pub(crate) fn normalize(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
