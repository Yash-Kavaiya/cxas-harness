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

use std::borrow::Cow;

/// Evaluation run lifecycle state, including unknown wire values (#284).
///
/// Unknown integers map to [`EvaluationRunState::Unknown`] so callers never
/// panic when the wire enum grows beyond this crate's known set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvaluationRunState {
    Unspecified,
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Unknown(i32),
}

impl EvaluationRunState {
    /// Map a protobuf wire integer to a typed state without panicking.
    pub fn from_wire(value: i32) -> Self {
        match value {
            0 => Self::Unspecified,
            1 => Self::Pending,
            2 => Self::Running,
            3 => Self::Succeeded,
            4 => Self::Failed,
            5 => Self::Cancelled,
            other => Self::Unknown(other),
        }
    }

    /// Stable string name for logs and diagnostics.
    ///
    /// Known variants use proto enum names; unknown wire values use
    /// `UNKNOWN(n)` rather than Python-style name lookup on a raw `i32`.
    pub fn as_str_name(&self) -> Cow<'static, str> {
        match self {
            Self::Unspecified => Cow::Borrowed("EVALUATION_RUN_STATE_UNSPECIFIED"),
            Self::Pending => Cow::Borrowed("PENDING"),
            Self::Running => Cow::Borrowed("RUNNING"),
            Self::Succeeded => Cow::Borrowed("SUCCEEDED"),
            Self::Failed => Cow::Borrowed("FAILED"),
            Self::Cancelled => Cow::Borrowed("CANCELLED"),
            Self::Unknown(n) => Cow::Owned(format!("UNKNOWN({n})")),
        }
    }
}
