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

use thiserror::Error;

#[derive(Debug, Error)]
pub enum LintError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("unknown rule id {0}")]
    UnknownRule(String),
    #[error("missing API key in env {0}")]
    MissingApiKey(&'static str),
    #[error("model output was not JSON diagnostics")]
    UnparseableModel,
    #[error("gemini http {status}: {body}")]
    Http { status: u16, body: String },
}
