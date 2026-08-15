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

pub mod actions;
pub mod deploy;
pub mod diff;
pub mod evals;
pub mod lint;
pub mod pull;
pub mod state;
pub mod trace;

use crate::output::{write_err, OutputFormat};
use clap::ArgMatches;
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn dispatch(matches: &ArgMatches, out: &mut impl Write) -> i32 {
    let format = OutputFormat::parse(matches.get_one::<String>("format").map(String::as_str));
    match matches.subcommand() {
        Some(("lint", sub)) => lint::run(sub, format, out),
        Some(("pull", sub)) => pull::run(sub, format, out),
        Some(("actions", sub)) => match sub.subcommand() {
            Some(("init", init)) => actions::run(init, format, out),
            _ => write_err(
                out,
                format,
                "actions",
                "USAGE",
                "expected actions init",
                2,
            ),
        },
        Some(("init-github-action", sub)) => actions::run(sub, format, out),
        Some(("trace", sub)) => trace::run(sub, format, out),
        Some(("evals", sub)) => match sub.subcommand() {
            Some(("report", report)) => evals::run(report, format, out),
            _ => write_err(out, format, "evals", "USAGE", "expected evals report", 2),
        },
        Some(("deploy", sub)) => deploy::run(sub, format, out),
        Some(("diff", sub)) => diff::run(sub, format, out),
        Some(("state", sub)) => state::run(sub, format, out),
        Some((name, _)) => write_err(
            out,
            format,
            name,
            "NOT_IMPLEMENTED",
            &format!("{name} lands in this crate after its owning phase"),
            1,
        ),
        None => write_err(
            out,
            format,
            "cxas",
            "USAGE",
            "a subcommand is required",
            2,
        ),
    }
}

pub fn opt_str(matches: &ArgMatches, name: &str) -> Option<String> {
    matches.get_one::<String>(name).cloned()
}

pub fn resolve_location(matches: &ArgMatches, search: Option<&Path>) -> Option<String> {
    if let Some(loc) = opt_str(matches, "location") {
        if !loc.trim().is_empty() && loc.trim() != "__default_global__" {
            return Some(loc);
        }
    }
    let start = search
        .map(Path::to_path_buf)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    match cxas_state::resolve_workspace(&start) {
        Ok(ws) => Some(ws.location.as_str().to_string()),
        Err(_) => None,
    }
}
