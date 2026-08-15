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

#[tokio::test]
async fn pull_forwards_version_id_to_transport() {
    let rec = cxas_cli::test_support::RecordingTransport::default();
    cxas_cli::set_transport_for_test(rec.clone());
    rec.stub_export(vec![0u8; 5 * 1024 * 1024]);
    let dir = tempfile::tempdir().unwrap();
    let mut buf = Cursor::new(Vec::new());
    let code = cxas_cli::run(
        &[
            "cxas".into(),
            "pull".into(),
            "--app".into(),
            "projects/p/locations/us/apps/a".into(),
            "--location".into(),
            "us".into(),
            "--target-dir".into(),
            dir.path().display().to_string(),
            "--version-id".into(),
            "v3".into(),
        ],
        &mut buf,
    );
    assert_eq!(code, 0);
    assert_eq!(rec.last_export_version().as_deref(), Some("v3"));
    assert!(rec.last_export_bytes() >= 5 * 1024 * 1024);
}
