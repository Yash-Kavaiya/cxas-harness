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

use cxas_core::{ClientConfig, CoreError, Credentials, Location};

#[test]
fn empty_location_is_rejected() {
    let err = Location::new("  ").unwrap_err();
    assert!(matches!(err, CoreError::LocationRequired));
}

#[test]
fn implicit_global_sentinel_is_rejected() {
    let err = Location::new("__default_global__").unwrap_err();
    assert!(matches!(err, CoreError::LocationHardcodedGlobalForbidden));
}

#[test]
fn explicit_global_is_allowed() {
    let loc = Location::new("global").unwrap();
    assert_eq!(loc.as_str(), "global");
}

#[test]
fn client_config_stores_the_given_location() {
    let cfg = ClientConfig {
        project_id: "demo".into(),
        location: Location::new("europe-west1").unwrap(),
        credentials: Credentials::ApplicationDefault,
    };
    assert_eq!(cfg.location.as_str(), "europe-west1");
}
