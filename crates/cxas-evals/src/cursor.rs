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

use bytes::Bytes;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserInput {
    Text(String),
    Dtmf(String),
    Audio(Bytes),
}

impl UserInput {
    pub fn display_text(&self) -> String {
        match self {
            UserInput::Text(s) | UserInput::Dtmf(s) => s.clone(),
            UserInput::Audio(_) => "<audio>".into(),
        }
    }
}

pub struct TurnCursor {
    utterances: Vec<UserInput>,
    index: usize,
}

impl TurnCursor {
    pub fn new(utterances: Vec<UserInput>) -> Self {
        Self {
            utterances,
            index: 0,
        }
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

    pub fn index(&self) -> usize {
        self.index
    }
}
