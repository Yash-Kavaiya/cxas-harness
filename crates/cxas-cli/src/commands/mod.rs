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
pub mod migrate;
pub mod pull;
pub mod resources;
pub mod state;
pub mod trace;

use crate::output::{write_err, write_ok, OutputFormat};
use clap::ArgMatches;
use std::io::{IsTerminal, Write};
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
        Some(("migrate", sub)) => match sub.subcommand() {
            Some(("dfcx", dfcx)) => migrate::run(dfcx, format, out),
            _ => write_err(
                out,
                format,
                "migrate",
                "USAGE",
                "expected migrate dfcx",
                2,
            ),
        },
        Some(("run-session", _)) => run_session(format, out),
        Some(("llm-lint", sub)) => llm_lint(sub, format, out),
        Some(("help", _)) => write_ok_help(format, out),
        Some((name, sub)) => match sub.subcommand() {
            Some((child, child_m)) => {
                resources::dispatch(&format!("{name} {child}"), child_m, format, out)
            }
            None => resources::dispatch(name, sub, format, out),
        },
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

fn not_implemented(out: &mut impl Write, format: OutputFormat, command: &str) -> i32 {
    write_err(
        out,
        format,
        command,
        "NOT_IMPLEMENTED",
        &format!("{command} lands in this crate after its owning phase"),
        1,
    )
}

fn run_session(format: OutputFormat, out: &mut impl Write) -> i32 {
    if !std::io::stdin().is_terminal() {
        return write_err(
            out,
            format,
            "run-session",
            "TTY_REQUIRED",
            "run-session requires a TTY",
            2,
        );
    }
    not_implemented(out, format, "run-session")
}

fn llm_lint(_matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    #[cfg(not(feature = "llm"))]
    {
        return write_err(
            out,
            format,
            "llm-lint",
            "FEATURE_DISABLED",
            "llm-lint requires --features llm",
            2,
        );
    }
    #[cfg(feature = "llm")]
    {
        not_implemented(out, format, "llm-lint")
    }
}

fn write_ok_help(format: OutputFormat, out: &mut impl Write) -> i32 {
    write_ok(
        out,
        format,
        "help",
        serde_json::json!({ "binary": "cxas" }),
        "cxas --help",
    )
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
