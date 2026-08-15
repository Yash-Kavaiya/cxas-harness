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

use cxas_discovery::{Discovery, DiscoveryError};
use std::io::Write;

const FIXTURE: &str = r#"{
  "revision": "20260101",
  "version": "v1test",
  "schemas": {
    "EvaluationRun": {
      "id": "EvaluationRun",
      "properties": {
        "state": { "type": "string", "enum": ["A_UNSPECIFIED", "QUEUED", "DONE"] },
        "name": { "type": "string" }
      }
    }
  },
  "resources": {
    "projects": {
      "resources": {
        "locations": {
          "methods": {
            "get": { "id": "ces.projects.locations.get", "httpMethod": "GET", "path": "v1/{+name}" }
          }
        }
      },
      "methods": {
        "list": { "id": "ces.projects.list", "httpMethod": "GET", "path": "v1/projects" }
      }
    }
  }
}"#;

fn write_fixture(body: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().expect("tempfile");
    f.write_all(body.as_bytes()).expect("write");
    f.flush().expect("flush");
    f
}

#[test]
fn parses_revision() {
    let f = write_fixture(FIXTURE);
    let d = Discovery::load(f.path()).expect("parse");
    assert_eq!(d.revision(), "20260101");
}

#[test]
fn walks_nested_resources_for_methods() {
    let f = write_fixture(FIXTURE);
    let d = Discovery::load(f.path()).expect("parse");
    let mut ids: Vec<_> = d.methods().map(|m| m.id.as_str()).collect();
    ids.sort();
    assert_eq!(ids, vec!["ces.projects.list", "ces.projects.locations.get"]);
}

#[test]
fn method_lookup_returns_verb_and_path() {
    let f = write_fixture(FIXTURE);
    let d = Discovery::load(f.path()).expect("parse");
    let m = d.method("ces.projects.locations.get").expect("method");
    assert_eq!(m.http_method, "GET");
    assert_eq!(m.path, "v1/{+name}");
    assert!(d.method("ces.does.not.exist").is_none());
}

#[test]
fn enum_field_lookup_returns_values_in_order() {
    let f = write_fixture(FIXTURE);
    let d = Discovery::load(f.path()).expect("parse");
    let e = d.enum_field("EvaluationRun", "state").expect("enum field");
    assert_eq!(e.values, vec!["A_UNSPECIFIED", "QUEUED", "DONE"]);
}

#[test]
fn non_enum_property_is_not_an_enum_field() {
    let f = write_fixture(FIXTURE);
    let d = Discovery::load(f.path()).expect("parse");
    assert!(d.enum_field("EvaluationRun", "name").is_none());
}

#[test]
fn missing_file_is_io_error() {
    let err = Discovery::load(std::path::Path::new("no/such/file.json")).unwrap_err();
    assert!(matches!(err, DiscoveryError::Io(_)));
}

#[test]
fn malformed_json_is_parse_error() {
    let f = write_fixture("{ not json");
    let err = Discovery::load(f.path()).unwrap_err();
    assert!(matches!(err, DiscoveryError::Parse(_)));
}
