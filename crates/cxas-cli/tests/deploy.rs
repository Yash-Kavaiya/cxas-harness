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

use std::io::Cursor;

#[test]
fn deploy_calls_import_version_and_deployment() {
    let rec = cxas_cli::test_support::RecordingTransport::default();
    cxas_cli::set_transport_for_test(rec.clone());
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("app.yaml"), "display_name: d\nroot_agent: main\n").unwrap();
    let mut buf = Cursor::new(Vec::new());
    let code = cxas_cli::run(
        &[
            "cxas".into(),
            "deploy".into(),
            "--app-dir".into(),
            dir.path().display().to_string(),
            "--project-id".into(),
            "p".into(),
            "--location".into(),
            "us".into(),
            "--channel-type".into(),
            "API".into(),
        ],
        &mut buf,
    );
    assert_eq!(code, 0);
    assert!(rec.imported());
    assert!(rec.version_created());
    assert!(rec.deployment_created());
}
