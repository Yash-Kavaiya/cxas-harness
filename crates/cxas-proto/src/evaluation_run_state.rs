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

/// Evaluation run lifecycle state, mirroring `EvaluationRun.state` in the
/// vendored CES `v1beta` discovery document.
///
/// Unknown wire values map to [`EvaluationRunState::Unknown`] so callers never
/// panic when the server's enum grows beyond this build's known set (#284).
/// Variant spellings are asserted against discovery by `cxas-parity`'s
/// `enum_variants_match_discovery`, which is what caught this enum declaring
/// `PENDING`/`SUCCEEDED`/`FAILED` — names CES has never used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluationRunState {
    Unspecified,
    Queued,
    Running,
    Completed,
    Error,
    Cancelled,
    /// A wire value this build does not know. Carries the raw value verbatim.
    Unknown(String),
}

impl EvaluationRunState {
    /// Map a REST/JSON wire string to a typed state without panicking.
    ///
    /// This is the canonical constructor: the CES REST surface encodes enums as
    /// strings, so an unrecognized value is a string, not an integer.
    pub fn from_wire_name(name: &str) -> Self {
        match name {
            "EVALUATION_RUN_STATE_UNSPECIFIED" => Self::Unspecified,
            "QUEUED" => Self::Queued,
            "RUNNING" => Self::Running,
            "COMPLETED" => Self::Completed,
            "ERROR" => Self::Error,
            "CANCELLED" => Self::Cancelled,
            other => Self::Unknown(other.to_string()),
        }
    }

    /// Map a protobuf wire integer by discovery declaration order.
    ///
    /// Retained for proto interop. Out-of-range values are preserved verbatim
    /// rather than looked up by `.name` on a raw integer, which is the exact
    /// Python crash this type exists to prevent (#284).
    pub fn from_wire(value: i32) -> Self {
        match value {
            0 => Self::Unspecified,
            1 => Self::Queued,
            2 => Self::Running,
            3 => Self::Completed,
            4 => Self::Error,
            5 => Self::Cancelled,
            other => Self::Unknown(other.to_string()),
        }
    }

    /// Stable wire spelling for logs, diagnostics, and JSON round-trips.
    pub fn as_str_name(&self) -> Cow<'static, str> {
        match self {
            Self::Unspecified => Cow::Borrowed("EVALUATION_RUN_STATE_UNSPECIFIED"),
            Self::Queued => Cow::Borrowed("QUEUED"),
            Self::Running => Cow::Borrowed("RUNNING"),
            Self::Completed => Cow::Borrowed("COMPLETED"),
            Self::Error => Cow::Borrowed("ERROR"),
            Self::Cancelled => Cow::Borrowed("CANCELLED"),
            Self::Unknown(raw) => Cow::Owned(format!("UNKNOWN({raw})")),
        }
    }

    /// True when the run reached a terminal state.
    ///
    /// Pollers must treat an unknown state as non-terminal: a future CES state
    /// this build has never seen is not evidence the run has finished.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Error | Self::Cancelled)
    }
}
