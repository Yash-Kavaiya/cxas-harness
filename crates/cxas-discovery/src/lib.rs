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

//! Pure parser over vendored CES discovery documents.
//!
//! No network, no code generation, no CES semantics. This crate is the single
//! definition of "what the API is"; the parity tests and the Gauntlet evidence
//! collector both query through it so they cannot drift from one another.
//!
//! Because it is the only lens onto the reference, anything it cannot see is
//! unverifiable everywhere else. CES declares enum values in three places, and
//! all three are surfaced:
//!
//! - `schemas.<S>.properties.<p>.enum` -- a scalar enum property
//! - `schemas.<S>.properties.<p>.items.enum` -- a repeated enum property
//! - `resources..methods.<m>.parameters.<p>.enum` -- an enum query parameter
//!
//! The first two are [`EnumField`]; the third is [`ParameterEnum`], kept apart
//! because a query parameter has no schema behind it to key on.

mod model;
mod parse;

pub use model::{Discovery, EnumField, Method, ParameterEnum};

/// Failure modes when loading a discovery document.
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("reading discovery document: {0}")]
    Io(#[from] std::io::Error),
    #[error("parsing discovery document: {0}")]
    Parse(#[from] serde_json::Error),
    /// Valid JSON that is not a usable discovery document. Reported rather
    /// than tolerated: a model with nothing in it satisfies every coverage
    /// and parity assertion vacuously.
    #[error("not a usable discovery document: {0}")]
    Malformed(String),
}

/// Crate name, mirroring the convention used by the sibling crates.
pub fn crate_name() -> &'static str {
    "cxas-discovery"
}
