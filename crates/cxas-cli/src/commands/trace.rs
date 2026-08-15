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

use crate::output::{write_ok, OutputFormat};
use crate::transport::take_scripted_trace;
use clap::ArgMatches;
use serde_json::json;
use std::io::Write;

pub fn run(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let raw = matches.get_flag("raw") || format == OutputFormat::Json;
    let scripted = take_scripted_trace();
    let turns: Vec<serde_json::Value> = scripted
        .into_iter()
        .enumerate()
        .map(|(i, proto)| {
            if raw {
                json!({
                    "turn": i,
                    "user": {},
                    "agent": proto,
                    "raw": proto,
                })
            } else {
                json!({
                    "turn": i,
                    "user": {},
                    "agent": proto,
                })
            }
        })
        .collect();
    write_ok(
        out,
        format,
        "trace",
        json!({ "turns": turns }),
        &format!("{} turn(s)", turns.len()),
    )
}
