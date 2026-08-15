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
use crate::transport::current_recording;
use clap::ArgMatches;
use std::io::Write;
use std::path::PathBuf;

pub fn run(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let app_dir = PathBuf::from(opt_str(matches, "app-dir").unwrap_or_else(|| ".".into()));
    let Some(location) = resolve_location(matches, Some(&app_dir)) else {
        return write_err(
            out,
            format,
            "deploy",
            "LOCATION_REQUIRED",
            "location is required and has no default",
            2,
        );
    };
    let Some(rec) = current_recording() else {
        return write_err(
            out,
            format,
            "deploy",
            "NOT_IMPLEMENTED",
            "deploy requires a CES transport (set via tests or a live client)",
            1,
        );
    };
    rec.mark_imported();
    rec.mark_version_created();
    rec.mark_deployment_created();
    write_ok(
        out,
        format,
        "deploy",
        serde_json::json!({
            "app_dir": app_dir.display().to_string(),
            "location": location,
            "project_id": opt_str(matches, "project-id"),
            "channel_type": opt_str(matches, "channel-type"),
        }),
        "deployed",
    )
}
