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

use std::fs;
use std::io::Cursor;

#[test]
fn state_prints_hash_and_location() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("cxas.workspace.yaml"),
        "profiles:\n  x:\n    project_id: p\n    location: europe-west1\nactive: x\n",
    )
    .unwrap();
    fs::write(dir.path().join("app.yaml"), "display_name: d\n").unwrap();
    let mut buf = Cursor::new(Vec::new());
    let code = cxas_cli::run(
        &[
            "cxas".into(),
            "state".into(),
            "--app-dir".into(),
            dir.path().display().to_string(),
        ],
        &mut buf,
    );
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_slice(buf.get_ref()).unwrap();
    assert!(v["data"]["hash"].as_str().unwrap().len() == 64);
    assert_eq!(v["data"]["profile"]["location"], "europe-west1");
}

#[test]
fn diff_exits_one_on_drift() {
    let rec = cxas_cli::test_support::RecordingTransport::default();
    rec.stub_remote_tree(&[("tools/only-remote.yaml", "x")]);
    cxas_cli::set_transport_for_test(rec);
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("app.yaml"), "display_name: d\n").unwrap();
    let mut buf = Cursor::new(Vec::new());
    let code = cxas_cli::run(
        &[
            "cxas".into(),
            "diff".into(),
            "--app-dir".into(),
            dir.path().display().to_string(),
            "--location".into(),
            "us".into(),
            "--app".into(),
            "projects/p/locations/us/apps/a".into(),
        ],
        &mut buf,
    );
    assert_eq!(code, 1);
    let v: serde_json::Value = serde_json::from_slice(buf.get_ref()).unwrap();
    assert_eq!(v["error"]["code"], "DRIFT");
}
