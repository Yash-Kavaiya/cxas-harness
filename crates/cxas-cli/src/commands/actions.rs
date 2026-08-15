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
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const TEST_TMPL: &str = include_str!("../../templates/test_workflow.yml.tmpl");
const CLEANUP_TMPL: &str = include_str!("../../templates/cleanup_workflow.yml.tmpl");

pub fn run(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    if matches.get_flag("auto-create-wif") {
        return write_err(
            out,
            format,
            "actions init",
            "WIF_MANUAL",
            "--auto-create-wif is not implemented; configure Workload Identity Federation manually",
            2,
        );
    }

    let app_dir = PathBuf::from(opt_str(matches, "app-dir").unwrap_or_else(|| ".".into()));
    let envs = read_environments(&app_dir);
    let agent = sanitize_agent(&read_display_name(&app_dir));
    let matrix = envs.join(", ");

    let test_yaml = TEST_TMPL
        .replace("{{ENVIRONMENTS}}", &matrix)
        .replace("{{AGENT}}", &agent);
    let cleanup_yaml = CLEANUP_TMPL
        .replace("{{ENVIRONMENTS}}", &matrix)
        .replace("{{AGENT}}", &agent);

    let wf_dir = app_dir.join(".github").join("workflows");
    if let Err(err) = fs::create_dir_all(&wf_dir) {
        return write_err(out, format, "actions init", "IO", &err.to_string(), 1);
    }
    let test_path = wf_dir.join(format!("test_{agent}.yml"));
    if let Err(err) = fs::write(&test_path, test_yaml) {
        return write_err(out, format, "actions init", "IO", &err.to_string(), 1);
    }

    let mut written = vec![test_path.display().to_string()];
    if !matches.get_flag("no-cleanup") {
        let cleanup_path = wf_dir.join(format!("cleanup_{agent}.yml"));
        if let Err(err) = fs::write(&cleanup_path, cleanup_yaml) {
            return write_err(out, format, "actions init", "IO", &err.to_string(), 1);
        }
        written.push(cleanup_path.display().to_string());
    }

    write_ok(
        out,
        format,
        "actions init",
        serde_json::json!({
            "agent": agent,
            "environments": envs,
            "written": written,
        }),
        &format!("wrote workflows for {agent}"),
    )
}

fn read_environments(app_dir: &Path) -> Vec<String> {
    let path = app_dir.join("environment.json");
    let Ok(text) = fs::read_to_string(path) else {
        return vec!["default".into()];
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return vec!["default".into()];
    };
    match value {
        serde_json::Value::Object(map) if !map.is_empty() => map.keys().cloned().collect(),
        _ => vec!["default".into()],
    }
}

fn read_display_name(app_dir: &Path) -> String {
    for name in ["app.yaml", "app.yml", "app.json"] {
        let path = app_dir.join(name);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        if name.ends_with(".json") {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(n) = v.get("display_name").and_then(|x| x.as_str()) {
                    return n.to_string();
                }
            }
        } else if let Ok(v) = serde_yaml::from_str::<serde_yaml::Value>(&text) {
            if let Some(n) = v.get("display_name").and_then(|x| x.as_str()) {
                return n.to_string();
            }
        }
    }
    "app".into()
}

fn sanitize_agent(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == '_' {
            out.push('_');
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "app".into()
    } else {
        out
    }
}
