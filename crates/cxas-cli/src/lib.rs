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

mod args;
mod commands;
mod output;

pub use args::build_parser;

use crate::output::{write_err, OutputFormat};
use std::io::Write;

pub fn crate_name() -> &'static str {
    "cxas-cli"
}

pub fn run(argv: &[String], out: &mut impl Write) -> i32 {
    let parser = build_parser();
    match parser.try_get_matches_from(argv.iter()) {
        Ok(matches) => commands::dispatch(&matches, out),
        Err(err) => {
            use clap::error::ErrorKind;
            match err.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                    let _ = write!(out, "{err}");
                    0
                }
                _ => write_err(
                    out,
                    OutputFormat::Json,
                    command_hint(argv),
                    "USAGE",
                    &err.to_string(),
                    2,
                ),
            }
        }
    }
}

fn command_hint(argv: &[String]) -> &str {
    argv.iter()
        .skip(1)
        .find(|a| !a.starts_with('-'))
        .map(String::as_str)
        .unwrap_or("cxas")
}
