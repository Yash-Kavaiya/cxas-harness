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

//! The vendored documents are an artifact somebody fetched from CES, not a
//! source file. `PINNED.toml` records which artifact that was. Nothing used to
//! check the two against each other, so a truncated or hand-edited reference
//! would have kept every count, census, and parity assertion green while
//! measuring a file nobody vouched for.

use cxas_discovery::{Discovery, DiscoveryError};
use std::path::{Path, PathBuf};

fn reference_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../reference/ces")
}

fn read(name: &str) -> String {
    std::fs::read_to_string(reference_dir().join(name)).expect("reference must be readable")
}

/// Lay out a reference directory whose contents we can perturb one field at a
/// time. Only the rejection paths use this; the acceptance path below is the
/// one that binds this crate to what CES actually served.
fn staged(doc: &str, pin: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("v1.discovery.json"), doc).expect("write doc");
    std::fs::write(dir.path().join("PINNED.toml"), pin).expect("write pin");
    dir
}

fn expect_pin_error(dir: &Path, version: &str) -> String {
    match Discovery::load_pinned(dir, version) {
        Err(DiscoveryError::Pin { detail, .. }) => detail,
        Err(other) => panic!("expected a pin failure, got {other}"),
        Ok(_) => panic!("expected a pin failure, got a document"),
    }
}

#[test]
fn vendored_documents_match_their_recorded_pin() {
    // The whole suite reads these two files. If they are not the artifact
    // `tools/refresh_reference.py` fetched and recorded, nothing else it
    // asserts about "what CES declares" means anything.
    for version in ["v1", "v1beta"] {
        let d = Discovery::load_pinned(&reference_dir(), version)
            .unwrap_or_else(|e| panic!("vendored {version} must match its pin: {e}"));
        assert_eq!(d.version(), version);
    }
}

#[test]
fn a_hand_edited_vendored_document_is_rejected() {
    // One enum value flipped to another of the same length, nothing else: the
    // sha256 is the only thing that can notice, because the document still
    // parses, still counts the same, and is still the same size.
    let original = read("v1.discovery.json");
    let doc = original.replace("\"LINEAR16\"", "\"INACTIVE\"");
    assert_ne!(
        doc, original,
        "the mutation must actually apply, or this test proves nothing"
    );
    assert_eq!(doc.len(), original.len(), "edit must be length-preserving");

    let dir = staged(&doc, &read("PINNED.toml"));
    assert!(expect_pin_error(dir.path(), "v1").contains("sha256"));
}

#[test]
fn a_truncated_vendored_document_is_rejected() {
    let full = read("v1.discovery.json");
    let dir = staged(&full[..full.len() / 2], &read("PINNED.toml"));
    assert!(expect_pin_error(dir.path(), "v1").contains("sha256"));
}

#[test]
fn a_document_whose_revision_disagrees_with_the_pin_is_rejected() {
    // Bytes intact, pin re-stamped: catches a PINNED.toml edited by hand to
    // claim currency the vendored copy does not have.
    let pin = read("PINNED.toml").replacen("20260806", "20261231", 1);
    let dir = staged(&read("v1.discovery.json"), &pin);
    let detail = expect_pin_error(dir.path(), "v1");
    assert!(detail.contains("20260806") && detail.contains("20261231"), "{detail}");
}

#[test]
fn an_unpinned_version_is_rejected() {
    let dir = staged(&read("v1.discovery.json"), &read("PINNED.toml"));
    assert!(expect_pin_error(dir.path(), "v2").contains("PINNED.toml"));
}
