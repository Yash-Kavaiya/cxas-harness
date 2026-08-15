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

use crate::commands::{opt_str, resolve_location};
use crate::output::{write_err, write_ok, OutputFormat};
use crate::transport::{block_on, ces_transport};
use clap::ArgMatches;
use cxas_core::{AppName, Apps, ClientConfig, CoreError, Credentials, Location};
use futures::StreamExt;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub fn run(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let Some(location) = resolve_location(matches, None) else {
        return write_err(
            out,
            format,
            "pull",
            "LOCATION_REQUIRED",
            "location is required and has no default",
            2,
        );
    };
    let app = opt_str(matches, "app").unwrap_or_default();
    let target = opt_str(matches, "target-dir").unwrap_or_else(|| ".".into());
    let version_id = opt_str(matches, "version-id");
    let loc = match Location::new(&location) {
        Ok(loc) => loc,
        Err(_) => {
            return write_err(
                out,
                format,
                "pull",
                "LOCATION_REQUIRED",
                "location is required and has no default",
                2,
            );
        }
    };
    let name = match AppName::parse(&app) {
        Ok(name) => name,
        Err(err) => {
            return write_err(out, format, "pull", "USAGE", &err.to_string(), 2);
        }
    };
    let config = ClientConfig {
        project_id: opt_str(matches, "project-id").unwrap_or_default(),
        location: loc,
        credentials: Credentials::ApplicationDefault,
    };
    let apps = Apps::new(config, ces_transport());
    let version_for_export = version_id.clone();
    let result = block_on(async move {
        let handle = if let Some(version) = version_for_export {
            apps.export_app_version(&name, &version).await?
        } else {
            apps.export_app(&name).await?
        };
        let mut acc = Vec::new();
        let mut handle = handle;
        while let Some(chunk) = handle.next().await {
            acc.extend_from_slice(&chunk?);
        }
        Ok::<Vec<u8>, CoreError>(acc)
    });
    let bytes = match result {
        Ok(bytes) => bytes,
        Err(CoreError::NotFound(msg)) => {
            return write_err(out, format, "pull", "CES_NOT_FOUND", &msg, 1);
        }
        Err(err) => {
            return write_err(out, format, "pull", "CES_NOT_FOUND", &err.to_string(), 1);
        }
    };

    let dest = PathBuf::from(&target);
    if let Err(err) = fs::create_dir_all(&dest) {
        return write_err(out, format, "pull", "IO", &err.to_string(), 1);
    }
    if !bytes.is_empty() {
        if let Err(err) = fs::write(dest.join("export.bin"), &bytes) {
            return write_err(out, format, "pull", "IO", &err.to_string(), 1);
        }
    }

    write_ok(
        out,
        format,
        "pull",
        serde_json::json!({
            "app": app,
            "target_dir": target,
            "version_id": version_id,
            "bytes": bytes.len(),
        }),
        "pulled",
    )
}
