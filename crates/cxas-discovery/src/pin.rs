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

use crate::DiscoveryError;
use serde::Deserialize;
use std::fmt::Write as _;
use std::path::Path;

/// Name of the manifest `tools/refresh_reference.py` writes beside the
/// vendored documents.
pub(crate) const PINNED_FILE: &str = "PINNED.toml";

/// What `tools/refresh_reference.py` recorded about one vendored document.
///
/// The `url` field of the manifest is deliberately not modelled: provenance is
/// the refresh tool's concern, and this type exists only to answer "are these
/// the bytes that were fetched".
#[derive(Debug, Clone, Deserialize)]
pub struct PinnedReference {
    /// API version the document describes, e.g. `v1beta`.
    pub version: String,
    /// Upstream `revision` stamp carried by the document that was fetched.
    pub revision: String,
    /// Hex sha256 over the canonicalized bytes as written to disk.
    pub sha256: String,
}

/// The parsed `PINNED.toml` manifest.
#[derive(Debug, Clone, Deserialize)]
pub struct Pinned {
    #[serde(default, rename = "reference")]
    references: Vec<PinnedReference>,
}

impl Pinned {
    /// Read the pin manifest from a directory of vendored documents.
    pub fn load(dir: &Path) -> Result<Self, DiscoveryError> {
        let text = std::fs::read_to_string(dir.join(PINNED_FILE))?;
        toml::from_str(&text).map_err(|e| DiscoveryError::Malformed(format!("{PINNED_FILE}: {e}")))
    }

    pub fn references(&self) -> impl Iterator<Item = &PinnedReference> {
        self.references.iter()
    }

    pub fn reference(&self, version: &str) -> Option<&PinnedReference> {
        self.references.iter().find(|r| r.version == version)
    }
}

/// Lowercase hex sha256, matching the digest format the refresh tool records.
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut out = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

impl crate::Discovery {
    /// Load a vendored document only if it is the artifact `PINNED.toml` records.
    ///
    /// Checks are ordered so the message names the most specific cause: an
    /// unpinned version first, then the bytes, then the revision stamp. The
    /// sha256 is the only check that can notice an edit which preserves both
    /// length and parseability, such as one enum value swapped for another.
    pub fn load_pinned(dir: &Path, version: &str) -> Result<Self, DiscoveryError> {
        let fail = |detail: String| DiscoveryError::Pin {
            version: version.to_string(),
            detail,
        };

        let pinned = Pinned::load(dir)?;
        let Some(reference) = pinned.reference(version) else {
            return Err(fail(format!("{PINNED_FILE} records no entry for it")));
        };

        let path = dir.join(format!("{version}.discovery.json"));
        let bytes = std::fs::read(&path)?;
        let actual = sha256_hex(&bytes);
        if actual != reference.sha256 {
            return Err(fail(format!(
                "sha256 mismatch: {PINNED_FILE} records {}, file is {actual}",
                reference.sha256
            )));
        }

        let doc = Self::parse(&String::from_utf8_lossy(&bytes))?;
        if doc.revision() != reference.revision {
            return Err(fail(format!(
                "revision mismatch: document says {}, {PINNED_FILE} says {}",
                doc.revision(),
                reference.revision
            )));
        }

        Ok(doc)
    }
}
