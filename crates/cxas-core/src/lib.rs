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

mod apps;
mod config;
mod deployments;
mod error;
mod evaluations;
mod location;
pub mod rest;
pub mod parity_table;
mod transport;

pub use apps::{AppName, Apps};
pub use config::{ClientConfig, Credentials};
pub use deployments::{Deployment, DeploymentName, Deployments};
pub use error::CoreError;
pub use evaluations::{Evaluations, QuotaKind};
pub use location::Location;
pub use rest::{
    api_version_of, expand_path, method_spec, status_to_error, ApiVersion, AppRef, LocationRef,
    MethodSpec, RequestBuilder, RestRequest, UrlError, DEFAULT_ENDPOINT, METHODS,
};
#[cfg(feature = "rest")]
pub use rest::CesHttpClient;
pub use transport::{
    CesTransport, ChannelSettings, ExportHandle, ExportRequest, NoopTransport, RecordingTransport,
};
