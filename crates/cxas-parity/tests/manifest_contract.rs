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

use cxas_parity::{load_bundled, ParityError};

#[test]
fn bundled_manifest_loads_and_has_version_1() {
    let m = load_bundled().expect("bundled YAML must parse");
    assert_eq!(m.version, 1);
    assert_eq!(
        m.source.commit,
        "4f7b43ca6adda0acad95a7e3654eee4e2ed1438c"
    );
}

#[test]
fn missing_file_is_io_error() {
    let err = cxas_parity::load_manifest(std::path::Path::new(
        "this/path/does/not/exist.yaml",
    ))
    .unwrap_err();
    assert!(matches!(err, ParityError::Io(_)));
}
