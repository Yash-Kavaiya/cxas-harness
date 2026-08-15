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
use crate::output::{write_err, write_err_with_data, write_ok, OutputFormat};
use crate::transport::current_recording;
use clap::ArgMatches;
use cxas_state::{diff_trees, hash_app_dir, AppTree};
use std::io::Write;
use std::path::PathBuf;

pub fn run(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let app_dir = PathBuf::from(opt_str(matches, "app-dir").unwrap_or_else(|| ".".into()));
    if resolve_location(matches, Some(&app_dir)).is_none() {
        return write_err(
            out,
            format,
            "diff",
            "LOCATION_REQUIRED",
            "location is required and has no default",
            2,
        );
    }
    let local = match hash_app_dir(&app_dir) {
        Ok(tree) => tree,
        Err(err) => return write_err(out, format, "diff", "IO", &err.to_string(), 1),
    };
    let remote = current_recording()
        .and_then(|rec| rec.remote_tree())
        .unwrap_or_else(AppTree::empty);
    let diff = diff_trees(&local, &remote);
    let data = serde_json::json!({
        "added": paths(&diff.added),
        "removed": paths(&diff.removed),
        "changed": paths(&diff.changed),
    });
    if diff.is_empty() {
        return write_ok(out, format, "diff", data, "no drift");
    }
    if matches.get_flag("allow-drift") {
        return write_ok(out, format, "diff", data, "drift allowed");
    }
    write_err_with_data(
        out,
        format,
        "diff",
        "DRIFT",
        "local and remote app trees differ",
        Some(data),
        1,
    )
}

fn paths(items: &[PathBuf]) -> Vec<String> {
    items.iter().map(|p| p.display().to_string()).collect()
}
