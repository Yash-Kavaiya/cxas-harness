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

//! CES REST surface: declared methods, typed names, and request construction.
//!
//! Everything here is pure. Request building, URL expansion, and error mapping
//! are testable without a network or a credential, which is what lets the
//! parity suite check this crate's claims against the vendored discovery
//! documents instead of against a live service.

#[cfg(feature = "rest")]
mod http;
mod method;
mod names;
mod request;
mod url;

#[cfg(feature = "rest")]
pub use http::CesHttpClient;
pub use method::{method_spec, ApiVersion, MethodSpec, METHODS};
pub use names::{AppRef, LocationRef};
pub use request::{
    api_version_of, status_to_error, RequestBuilder, RestRequest, DEFAULT_ENDPOINT,
};
pub use url::{expand_path, UrlError};
