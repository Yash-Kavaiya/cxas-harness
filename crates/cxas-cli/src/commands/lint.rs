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
use crate::output::{write_err, write_err_with_data, write_ok, OutputFormat};
use clap::ArgMatches;
use cxas_lint::{discover, RuleRegistry};
use std::io::Write;
use std::path::PathBuf;

pub fn run(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let app_dir = opt_str(matches, "app-dir").unwrap_or_else(|| ".".into());
    let root = PathBuf::from(&app_dir);
    let ctx = match discover(&root) {
        Ok(ctx) => ctx,
        Err(err) => {
            return write_err(
                out,
                format,
                "lint",
                "LINT_IO",
                &err.to_string(),
                1,
            );
        }
    };
    let report = RuleRegistry::builtin().run_all(&ctx);
    let data = serde_json::json!({
        "diagnostics": report.diagnostics,
        "error_count": report.error_count(),
    });
    if report.error_count() == 0 {
        write_ok(
            out,
            format,
            "lint",
            data,
            &format!("lint ok ({} diagnostics)", report.diagnostics.len()),
        )
    } else {
        write_err_with_data(
            out,
            format,
            "lint",
            "LINT_ERRORS",
            &format!("{} lint error(s)", report.error_count()),
            Some(data),
            1,
        )
    }
}
