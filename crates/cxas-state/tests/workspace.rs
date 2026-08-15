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

use cxas_state::{resolve_workspace, StateError};
use std::fs;

#[test]
fn child_profile_overlays_parent_and_keeps_location() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("cxas.workspace.yaml"),
        r#"
profiles:
  base:
    project_id: parent-proj
    location: us-central1
  child:
    extends: base
    project_id: child-proj
active: child
"#,
    )
    .unwrap();
    let ws = resolve_workspace(tmp.path()).unwrap();
    assert_eq!(ws.project_id, "child-proj");
    assert_eq!(ws.location.as_str(), "us-central1");
}

#[test]
fn missing_location_is_error_not_global() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("cxas.workspace.yaml"),
        "profiles:\n  x:\n    project_id: p\nactive: x\n",
    )
    .unwrap();
    let err = resolve_workspace(tmp.path()).unwrap_err();
    assert!(matches!(err, StateError::LocationRequired));
}

#[test]
fn extends_cycle_is_error() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("cxas.workspace.yaml"),
        r#"
profiles:
  a:
    extends: b
    project_id: p
    location: us-central1
  b:
    extends: a
    project_id: q
    location: europe-west1
active: a
"#,
    )
    .unwrap();
    let err = resolve_workspace(tmp.path()).unwrap_err();
    assert!(matches!(err, StateError::ProfileCycle));
}
