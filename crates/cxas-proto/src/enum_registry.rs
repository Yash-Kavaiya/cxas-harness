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

//! Declares which discovery enum each Rust enum in this crate mirrors.
//!
//! `cxas-parity`'s `enum_variants_match_discovery` test walks this registry and
//! fails when a declared variant list diverges from the vendored CES document.
//! Adding a Rust enum without adding it here is caught by
//! `registry_covers_every_enum_in_cxas_proto`.

/// One Rust enum bound to its discovery source of truth.
#[derive(Debug, Clone, Copy)]
pub struct RegisteredEnum {
    /// Rust type name, used in assertion messages.
    pub rust_name: &'static str,
    /// Discovery schema id, e.g. `EvaluationRun`.
    pub schema: &'static str,
    /// Discovery property name, e.g. `state`.
    pub property: &'static str,
    /// Wire spellings this crate claims to implement, in discovery order.
    pub variants: &'static [&'static str],
    /// Discovery document that declares it: `"v1"` or `"v1beta"`.
    pub api_version: &'static str,
}

pub const REGISTERED_ENUMS: &[RegisteredEnum] = &[RegisteredEnum {
    rust_name: "EvaluationRunState",
    schema: "EvaluationRun",
    property: "state",
    api_version: "v1beta",
    variants: &[
        "EVALUATION_RUN_STATE_UNSPECIFIED",
        "QUEUED",
        "RUNNING",
        "COMPLETED",
        "ERROR",
        "CANCELLED",
    ],
}];
