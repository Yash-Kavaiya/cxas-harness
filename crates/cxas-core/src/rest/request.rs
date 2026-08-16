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

//! Building a CES request from a declared method.
//!
//! Request construction is pure and independent of any HTTP client, so the
//! whole URL, header, and body surface is testable without a network, a
//! credential, or a mock server. Only the send step needs the `rest` feature.

use super::method::{ApiVersion, MethodSpec};
use super::url::expand_path;
use crate::CoreError;
use std::collections::BTreeMap;

/// Default CES service host.
pub const DEFAULT_ENDPOINT: &str = "https://ces.googleapis.com";

/// A fully-formed CES request, not yet sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestRequest {
    pub http_method: &'static str,
    pub url: String,
    pub body: Option<String>,
    pub headers: Vec<(String, String)>,
}

/// Builds requests for one project, location, and credential.
#[derive(Debug, Clone)]
pub struct RequestBuilder {
    endpoint: String,
    token: Option<String>,
}

impl Default for RequestBuilder {
    fn default() -> Self {
        Self {
            endpoint: DEFAULT_ENDPOINT.to_string(),
            token: None,
        }
    }
}

impl RequestBuilder {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into().trim_end_matches('/').to_string(),
            token: None,
        }
    }

    /// Attach an OAuth bearer token.
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Build a request for `spec`, expanding its path template.
    ///
    /// `params` supplies the template variables; `query` becomes the query
    /// string, sorted so the output is byte-stable and therefore assertable.
    pub fn build(
        &self,
        spec: &MethodSpec,
        params: &BTreeMap<String, String>,
        query: &BTreeMap<String, String>,
        body: Option<String>,
    ) -> Result<RestRequest, CoreError> {
        let path = expand_path(spec.path, params)
            .map_err(|e| CoreError::Transport(format!("{}: {e}", spec.id)))?;

        let mut url = format!("{}/{}", self.endpoint, path);
        if !query.is_empty() {
            let encoded = query
                .iter()
                .map(|(k, v)| format!("{}={}", encode_query(k), encode_query(v)))
                .collect::<Vec<_>>()
                .join("&");
            url.push('?');
            url.push_str(&encoded);
        }

        let mut headers = vec![("accept".to_string(), "application/json".to_string())];
        if let Some(token) = &self.token {
            headers.push(("authorization".to_string(), format!("Bearer {token}")));
        }
        if body.is_some() {
            headers.push((
                "content-type".to_string(),
                "application/json".to_string(),
            ));
        }

        Ok(RestRequest {
            http_method: spec.http_method,
            url,
            body,
            headers,
        })
    }
}

/// Percent-encode a query key or value, including `/` and `:`.
fn encode_query(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// Map an HTTP status to a typed error, so callers can branch on the cause
/// rather than on a formatted string.
pub fn status_to_error(status: u16, body: &str) -> CoreError {
    let detail = body.trim();
    let detail = if detail.is_empty() {
        "<empty response body>"
    } else {
        detail
    };
    match status {
        401 | 403 => CoreError::Transport(format!("{status} not authorized: {detail}")),
        404 => CoreError::Transport(format!("404 not found: {detail}")),
        429 => CoreError::Transport(format!("429 quota exhausted: {detail}")),
        500..=599 => CoreError::Transport(format!("{status} CES server error: {detail}")),
        other => CoreError::Transport(format!("{other} unexpected CES response: {detail}")),
    }
}

/// Which API surface a method belongs to, for callers selecting an endpoint.
pub fn api_version_of(spec: &MethodSpec) -> ApiVersion {
    spec.api_version
}
