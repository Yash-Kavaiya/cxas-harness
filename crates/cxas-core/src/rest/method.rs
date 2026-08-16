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

//! The CES methods this crate can address, declared as data.
//!
//! The table itself lives in `method_table.rs` and is generated from the
//! vendored discovery documents, so it covers every method CES declares rather
//! than a hand-maintained subset that silently falls behind. `cxas-parity`'s
//! `declared_table_matches_discovery_exactly` fails if the two ever disagree
//! in either direction: a method CES added and the table lacks, or a method
//! the table claims and CES does not declare.
//!
//! Addressable is not the same as modelled. Every method here can be built and
//! sent; [`MODELLED`] names the smaller set this workspace wraps in its own
//! types and CLI verbs, and the two counts are reported separately so neither
//! flatters the other.

use super::method_table::METHODS;

/// Which CES API surface a method belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApiVersion {
    V1,
    V1Beta,
}

impl ApiVersion {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::V1 => "v1",
            Self::V1Beta => "v1beta",
        }
    }

    /// Parse a surface name as written on the command line or in a config.
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "v1" => Some(Self::V1),
            "v1beta" | "v1Beta" => Some(Self::V1Beta),
            _ => None,
        }
    }
}

/// One CES REST method, mirroring its discovery declaration exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MethodSpec {
    /// Discovery method id, e.g. `ces.projects.locations.apps.list`.
    pub id: &'static str,
    pub api_version: ApiVersion,
    pub http_method: &'static str,
    /// Discovery path template, verbatim, including its version prefix.
    pub path: &'static str,
}

impl MethodSpec {
    /// The template variables this method requires, in declaration order.
    ///
    /// Callers need this to report *which* parameter is missing before a
    /// request is built, rather than after the expansion fails.
    pub fn required_params(&self) -> Vec<&'static str> {
        let mut names = Vec::new();
        let mut rest = self.path;
        while let Some(open) = rest.find('{') {
            let Some(close) = rest[open..].find('}') else {
                break;
            };
            let raw = &rest[open + 1..open + close];
            names.push(raw.strip_prefix('+').unwrap_or(raw));
            rest = &rest[open + close + 1..];
        }
        names
    }

    /// True for the one method CES streams a response for.
    ///
    /// `streamRunSession` returns a JSON array delivered incrementally; a
    /// caller that buffers it to completion loses the property that makes it
    /// worth having.
    pub fn is_streaming(&self) -> bool {
        self.id.ends_with(".streamRunSession")
    }
}

/// Look up an addressable method by its discovery id and surface.
pub fn method_spec(id: &str, api_version: ApiVersion) -> Option<&'static MethodSpec> {
    METHODS
        .iter()
        .find(|m| m.id == id && m.api_version == api_version)
}

/// Look up a method by id on whichever surface declares it, preferring `v1`.
///
/// Most resources exist on both surfaces; the evaluation resources exist only
/// on `v1beta`. Preferring `v1` keeps a caller that names no version on the
/// stable surface wherever there is a choice.
pub fn resolve_method(id: &str) -> Option<&'static MethodSpec> {
    method_spec(id, ApiVersion::V1).or_else(|| method_spec(id, ApiVersion::V1Beta))
}

pub(super) const fn v1(id: &'static str, http_method: &'static str, path: &'static str) -> MethodSpec {
    MethodSpec {
        id,
        api_version: ApiVersion::V1,
        http_method,
        path,
    }
}

pub(super) const fn beta(
    id: &'static str,
    http_method: &'static str,
    path: &'static str,
) -> MethodSpec {
    MethodSpec {
        id,
        api_version: ApiVersion::V1Beta,
        http_method,
        path,
    }
}

/// The methods this workspace models with its own types, names, or CLI verbs,
/// as opposed to merely being able to address.
///
/// Kept separate from [`METHODS`] so the coverage report cannot claim credit
/// for generated breadth. Generating 170 path templates is cheap; deciding
/// what a `Deployment` is, and what happens when promoting one fails, is not.
pub const MODELLED: &[&str] = &[
    "ces.projects.locations.apps.list",
    "ces.projects.locations.apps.get",
    "ces.projects.locations.apps.create",
    "ces.projects.locations.apps.delete",
    "ces.projects.locations.apps.patch",
    "ces.projects.locations.apps.exportApp",
    "ces.projects.locations.apps.importApp",
    "ces.projects.locations.apps.agents.list",
    "ces.projects.locations.apps.agents.get",
    "ces.projects.locations.apps.agents.create",
    "ces.projects.locations.apps.agents.delete",
    "ces.projects.locations.apps.agents.patch",
    "ces.projects.locations.apps.tools.list",
    "ces.projects.locations.apps.tools.get",
    "ces.projects.locations.apps.tools.create",
    "ces.projects.locations.apps.tools.delete",
    "ces.projects.locations.apps.tools.patch",
    "ces.projects.locations.apps.versions.list",
    "ces.projects.locations.apps.versions.get",
    "ces.projects.locations.apps.versions.create",
    "ces.projects.locations.apps.deployments.list",
    "ces.projects.locations.apps.deployments.get",
    "ces.projects.locations.apps.deployments.create",
    "ces.projects.locations.apps.sessions.runSession",
    "ces.projects.locations.apps.sessions.streamRunSession",
    "ces.projects.locations.apps.evaluations.list",
    "ces.projects.locations.apps.evaluations.get",
    "ces.projects.locations.apps.evaluations.create",
    "ces.projects.locations.apps.evaluations.delete",
    "ces.projects.locations.apps.evaluations.patch",
    "ces.projects.locations.apps.evaluationRuns.list",
    "ces.projects.locations.apps.evaluationRuns.get",
    "ces.projects.locations.apps.evaluationRuns.delete",
    "ces.projects.locations.apps.evaluations.export",
    "ces.projects.locations.apps.evaluations.results.list",
    "ces.projects.locations.apps.evaluations.results.get",
    "ces.projects.locations.apps.runEvaluation",
];
