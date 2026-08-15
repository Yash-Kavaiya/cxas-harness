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

use serde::Serialize;
use serde_json::Value;
use std::io::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Human,
}

impl OutputFormat {
    pub fn parse(raw: Option<&str>) -> Self {
        match raw.map(str::trim) {
            Some(s) if s.eq_ignore_ascii_case("human") => Self::Human,
            _ => Self::Json,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct Envelope {
    pub ok: bool,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
}

pub fn write_ok(
    out: &mut impl Write,
    format: OutputFormat,
    command: &str,
    data: Value,
    human: &str,
) -> i32 {
    match format {
        OutputFormat::Json => {
            let env = Envelope {
                ok: true,
                command: command.to_string(),
                data: Some(data),
                error: None,
            };
            let _ = writeln!(
                out,
                "{}",
                serde_json::to_string(&env).unwrap_or_else(|_| r#"{"ok":true}"#.into())
            );
        }
        OutputFormat::Human => {
            let _ = writeln!(out, "{human}");
        }
    }
    0
}

pub fn write_err(
    out: &mut impl Write,
    format: OutputFormat,
    command: &str,
    code: &str,
    message: &str,
    exit: i32,
) -> i32 {
    write_err_with_data(out, format, command, code, message, None, exit)
}

pub fn write_err_with_data(
    out: &mut impl Write,
    format: OutputFormat,
    command: &str,
    code: &str,
    message: &str,
    data: Option<Value>,
    exit: i32,
) -> i32 {
    match format {
        OutputFormat::Json => {
            let env = Envelope {
                ok: false,
                command: command.to_string(),
                data,
                error: Some(ErrorBody {
                    code: code.to_string(),
                    message: message.to_string(),
                }),
            };
            let _ = writeln!(
                out,
                "{}",
                serde_json::to_string(&env).unwrap_or_else(|_| r#"{"ok":false}"#.into())
            );
        }
        OutputFormat::Human => {
            let _ = writeln!(out, "error {code}: {message}");
        }
    }
    exit
}
