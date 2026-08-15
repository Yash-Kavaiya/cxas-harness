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
use clap::ArgMatches;
use std::io::Write;

pub fn run(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    if resolve_location(matches, None).is_none() {
        return write_err(
            out,
            format,
            "pull",
            "LOCATION_REQUIRED",
            "location is required and has no default",
            2,
        );
    }
    write_ok(
        out,
        format,
        "pull",
        serde_json::json!({
            "app": opt_str(matches, "app"),
            "target_dir": opt_str(matches, "target-dir"),
            "version_id": opt_str(matches, "version-id"),
        }),
        "pulled",
    )
}
