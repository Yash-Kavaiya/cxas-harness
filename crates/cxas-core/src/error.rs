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
pub enum CoreError {
    #[error("location is required and has no default")]
    LocationRequired,
    #[error("refusing implicit global location sentinel")]
    LocationHardcodedGlobalForbidden,
    #[error("CES transport: {0}")]
    Transport(String),
    #[error("resource not found: {0}")]
    NotFound(String),
    #[error("invalid resource name: {0}")]
    InvalidName(String),
    #[error("export stream ended before content-length {expected} (got {got})")]
    TruncatedExport { expected: u64, got: u64 },
}
