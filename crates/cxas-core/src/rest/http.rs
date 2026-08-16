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

//! The one part of the REST layer that touches the network.
//!
//! Everything else in `rest` is pure, so this module stays deliberately thin:
//! it sends a [`RestRequest`] that has already been fully constructed and
//! checked, and maps the response status onto a typed error. Behind the `rest`
//! feature so a lint-or-parse-only build pulls in no HTTP stack at all.

use super::method::{ApiVersion, MethodSpec};
use super::request::{status_to_error, RequestBuilder, RestRequest};
use crate::CoreError;
use std::collections::BTreeMap;

/// A CES client that actually issues requests.
pub struct CesHttpClient {
    builder: RequestBuilder,
    http: reqwest::Client,
}

impl CesHttpClient {
    /// Build a client against the default CES endpoint.
    pub fn new(token: impl Into<String>) -> Result<Self, CoreError> {
        Self::with_builder(RequestBuilder::default().with_token(token))
    }

    /// Build a client against a specific endpoint, for tests or a private
    /// service attachment.
    pub fn with_builder(builder: RequestBuilder) -> Result<Self, CoreError> {
        let http = reqwest::Client::builder()
            .build()
            .map_err(|e| CoreError::Transport(format!("building HTTP client: {e}")))?;
        Ok(Self { builder, http })
    }

    pub fn endpoint(&self) -> &str {
        self.builder.endpoint()
    }

    /// Issue `spec` and return the response body on success.
    ///
    /// A non-2xx status becomes a typed [`CoreError`] carrying the status and
    /// the server's own message, so a 403 from a regional-residency policy is
    /// distinguishable from a 404 for a mistyped app id.
    pub async fn call(
        &self,
        spec: &MethodSpec,
        params: &BTreeMap<String, String>,
        query: &BTreeMap<String, String>,
        body: Option<String>,
    ) -> Result<String, CoreError> {
        let request = self.builder.build(spec, params, query, body)?;
        self.send(&request).await
    }

    /// Send an already-built request.
    pub async fn send(&self, request: &RestRequest) -> Result<String, CoreError> {
        let method = reqwest::Method::from_bytes(request.http_method.as_bytes())
            .map_err(|e| CoreError::Transport(format!("bad HTTP method: {e}")))?;

        let mut builder = self.http.request(method, &request.url);
        for (name, value) in &request.headers {
            builder = builder.header(name, value);
        }
        if let Some(body) = &request.body {
            builder = builder.body(body.clone());
        }

        let response = builder
            .send()
            .await
            .map_err(|e| CoreError::Transport(format!("sending request: {e}")))?;

        let status = response.status().as_u16();
        let text = response
            .text()
            .await
            .map_err(|e| CoreError::Transport(format!("reading response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(text)
        } else {
            Err(status_to_error(status, &text))
        }
    }

    /// Which surface a method targets, for callers that log or route on it.
    pub fn api_version(spec: &MethodSpec) -> ApiVersion {
        spec.api_version
    }
}
