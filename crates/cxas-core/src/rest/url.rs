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

//! Expansion of discovery path templates.
//!
//! Discovery writes paths as RFC 6570 templates, e.g. `v1/{+parent}/agents`.
//! The `+` marks *reserved* expansion: the value is a resource name whose
//! slashes are structural and must survive verbatim. A plain `{var}` is a
//! single opaque segment and is percent-encoded.
//!
//! Getting this backwards is silent and expensive: percent-encoding the
//! slashes in `projects/p/locations/us/apps/a` produces a URL the server
//! answers with 404, and no type in the workspace would object.

use std::collections::BTreeMap;

/// Why a template could not be turned into a path.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UrlError {
    #[error("path template references `{{{0}}}`, which was not supplied")]
    MissingParameter(String),
    #[error("path template is malformed: unclosed `{{` in `{0}`")]
    UnclosedBrace(String),
    #[error("parameter `{0}` must not be empty")]
    EmptyParameter(String),
}

/// Percent-encode one path segment, leaving RFC 3986 unreserved characters.
fn encode_segment(value: &str) -> String {
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

/// Percent-encode a reserved-expansion value, preserving path structure.
///
/// Slashes are structural here and are kept; everything else that is not
/// unreserved is encoded, so an app id containing a space or a `?` cannot
/// smuggle itself into the query string.
fn encode_reserved(value: &str) -> String {
    value
        .split('/')
        .map(encode_segment)
        .collect::<Vec<_>>()
        .join("/")
}

/// Expand a discovery path template against a parameter map.
///
/// ```
/// use cxas_core::expand_path;
/// use std::collections::BTreeMap;
///
/// let mut p = BTreeMap::new();
/// p.insert("parent".to_string(), "projects/x/locations/us/apps/a".to_string());
/// assert_eq!(
///     expand_path("v1/{+parent}/agents", &p).unwrap(),
///     "v1/projects/x/locations/us/apps/a/agents"
/// );
/// ```
pub fn expand_path(
    template: &str,
    params: &BTreeMap<String, String>,
) -> Result<String, UrlError> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            return Err(UrlError::UnclosedBrace(template.to_string()));
        };

        let token = &after[..close];
        let (name, reserved) = match token.strip_prefix('+') {
            Some(stripped) => (stripped, true),
            None => (token, false),
        };

        let value = params
            .get(name)
            .ok_or_else(|| UrlError::MissingParameter(name.to_string()))?;
        if value.is_empty() {
            return Err(UrlError::EmptyParameter(name.to_string()));
        }

        out.push_str(&if reserved {
            encode_reserved(value)
        } else {
            encode_segment(value)
        });

        rest = &after[close + 1..];
    }

    out.push_str(rest);
    Ok(out)
}
