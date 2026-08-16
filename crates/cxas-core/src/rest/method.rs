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

//! The CES methods this crate implements, declared as data.
//!
//! Every entry is checked against the vendored discovery documents by
//! `cxas-parity`'s `declared_methods_resolve_in_discovery`: a wrong verb, a
//! wrong path template, or an id CES does not define fails the build's test
//! suite rather than a live request.

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

/// Look up an implemented method by its discovery id and surface.
pub fn method_spec(id: &str, api_version: ApiVersion) -> Option<&'static MethodSpec> {
    METHODS
        .iter()
        .find(|m| m.id == id && m.api_version == api_version)
}

const fn v1(id: &'static str, http_method: &'static str, path: &'static str) -> MethodSpec {
    MethodSpec {
        id,
        api_version: ApiVersion::V1,
        http_method,
        path,
    }
}

const fn beta(id: &'static str, http_method: &'static str, path: &'static str) -> MethodSpec {
    MethodSpec {
        id,
        api_version: ApiVersion::V1Beta,
        http_method,
        path,
    }
}

/// Methods implemented by this crate.
///
/// Scoped deliberately: apps, agents, and tools on the stable `v1` surface,
/// plus the evaluation resources, which exist only on `v1beta`. The remaining
/// CES methods are unimplemented and are reported as such by the coverage
/// report rather than quietly claimed.
pub const METHODS: &[MethodSpec] = &[
    // ---- apps (v1) ----
    v1("ces.projects.locations.apps.list", "GET", "v1/{+parent}/apps"),
    v1("ces.projects.locations.apps.get", "GET", "v1/{+name}"),
    v1("ces.projects.locations.apps.create", "POST", "v1/{+parent}/apps"),
    v1("ces.projects.locations.apps.delete", "DELETE", "v1/{+name}"),
    v1("ces.projects.locations.apps.patch", "PATCH", "v1/{+name}"),
    v1("ces.projects.locations.apps.exportApp", "POST", "v1/{+name}:exportApp"),
    v1("ces.projects.locations.apps.importApp", "POST", "v1/{+parent}/apps:importApp"),
    // ---- agents (v1) ----
    v1("ces.projects.locations.apps.agents.list", "GET", "v1/{+parent}/agents"),
    v1("ces.projects.locations.apps.agents.get", "GET", "v1/{+name}"),
    v1("ces.projects.locations.apps.agents.create", "POST", "v1/{+parent}/agents"),
    v1("ces.projects.locations.apps.agents.delete", "DELETE", "v1/{+name}"),
    v1("ces.projects.locations.apps.agents.patch", "PATCH", "v1/{+name}"),
    // ---- tools (v1) ----
    v1("ces.projects.locations.apps.tools.list", "GET", "v1/{+parent}/tools"),
    v1("ces.projects.locations.apps.tools.get", "GET", "v1/{+name}"),
    v1("ces.projects.locations.apps.tools.create", "POST", "v1/{+parent}/tools"),
    v1("ces.projects.locations.apps.tools.delete", "DELETE", "v1/{+name}"),
    v1("ces.projects.locations.apps.tools.patch", "PATCH", "v1/{+name}"),
    // ---- versions (v1) ----
    v1("ces.projects.locations.apps.versions.list", "GET", "v1/{+parent}/versions"),
    v1("ces.projects.locations.apps.versions.get", "GET", "v1/{+name}"),
    v1("ces.projects.locations.apps.versions.create", "POST", "v1/{+parent}/versions"),
    // ---- deployments (v1) ----
    v1("ces.projects.locations.apps.deployments.list", "GET", "v1/{+parent}/deployments"),
    v1("ces.projects.locations.apps.deployments.get", "GET", "v1/{+name}"),
    v1("ces.projects.locations.apps.deployments.create", "POST", "v1/{+parent}/deployments"),
    // ---- evaluations (v1beta only) ----
    beta("ces.projects.locations.apps.evaluations.list", "GET", "v1beta/{+parent}/evaluations"),
    beta("ces.projects.locations.apps.evaluations.get", "GET", "v1beta/{+name}"),
    beta("ces.projects.locations.apps.evaluations.create", "POST", "v1beta/{+parent}/evaluations"),
    beta("ces.projects.locations.apps.evaluations.delete", "DELETE", "v1beta/{+name}"),
    beta("ces.projects.locations.apps.evaluations.patch", "PATCH", "v1beta/{+name}"),
    beta("ces.projects.locations.apps.evaluations.export", "POST", "v1beta/{+parent}/evaluations:export"),
    // ---- evaluation runs (v1beta only) ----
    beta("ces.projects.locations.apps.evaluationRuns.list", "GET", "v1beta/{+parent}/evaluationRuns"),
    beta("ces.projects.locations.apps.evaluationRuns.get", "GET", "v1beta/{+name}"),
    beta("ces.projects.locations.apps.evaluationRuns.delete", "DELETE", "v1beta/{+name}"),
    // ---- evaluation results (v1beta only) ----
    beta("ces.projects.locations.apps.evaluations.results.list", "GET", "v1beta/{+parent}/results"),
    beta("ces.projects.locations.apps.evaluations.results.get", "GET", "v1beta/{+name}"),
    // ---- run an evaluation (v1beta only) ----
    beta("ces.projects.locations.apps.runEvaluation", "POST", "v1beta/{+app}:runEvaluation"),
];
