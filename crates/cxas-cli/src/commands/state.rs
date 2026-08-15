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

use crate::commands::opt_str;
use crate::output::{write_err, write_ok, OutputFormat};
use clap::ArgMatches;
use cxas_state::{hash_app_dir, resolve_workspace};
use std::io::Write;
use std::path::PathBuf;

pub fn run(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let app_dir = PathBuf::from(opt_str(matches, "app-dir").unwrap_or_else(|| ".".into()));
    let profile = match resolve_workspace(&app_dir) {
        Ok(ws) => serde_json::json!({
            "name": ws.name,
            "project_id": ws.project_id,
            "location": ws.location.as_str(),
        }),
        Err(_) => {
            let location = opt_str(matches, "location");
            let project_id = opt_str(matches, "project-id").unwrap_or_default();
            match location {
                Some(loc) => serde_json::json!({
                    "name": "flags",
                    "project_id": project_id,
                    "location": loc,
                }),
                None => {
                    return write_err(
                        out,
                        format,
                        "state",
                        "LOCATION_REQUIRED",
                        "location is required and has no default",
                        2,
                    );
                }
            }
        }
    };
    let tree = match hash_app_dir(&app_dir) {
        Ok(tree) => tree,
        Err(err) => {
            return write_err(out, format, "state", "IO", &err.to_string(), 1);
        }
    };
    write_ok(
        out,
        format,
        "state",
        serde_json::json!({
            "hash": tree.root_hash.to_hex(),
            "profile": profile,
        }),
        &tree.root_hash.to_hex(),
    )
}
