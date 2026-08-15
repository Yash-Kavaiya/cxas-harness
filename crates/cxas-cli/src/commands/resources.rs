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

use crate::catalog::{self, AppRec, DeploymentRec};
use crate::commands::{opt_str, resolve_location};
use crate::output::{write_err, write_ok, OutputFormat};
use crate::transport::{block_on, current_recording};
use clap::ArgMatches;
use cxas_evals::{CallbackEvals, ToolEvals};
use cxas_lint::{discover, RuleRegistry};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub fn dispatch(
    command: &str,
    matches: &ArgMatches,
    format: OutputFormat,
    out: &mut impl Write,
) -> i32 {
    match command {
        "push" => push(matches, format, out),
        "create" => create(matches, format, out),
        "delete" => delete(matches, format, out),
        "init" => init(matches, format, out),
        "branch" => branch(matches, format, out),
        "apps list" => apps_list(matches, format, out),
        "apps get" => apps_get(matches, format, out),
        "conversations list" => conversations_list(matches, format, out),
        "conversations get" => conversations_get(matches, format, out),
        "deployments list" => deployments_list(matches, format, out),
        "deployments create" => deployments_create(matches, format, out),
        "deployments promote" => deployments_promote(matches, format, out),
        "local create" => local_create(matches, format, out),
        "versions list" => versions_list(matches, format, out),
        "versions compare" => versions_compare(matches, format, out),
        "insights" => insights(matches, format, out),
        "agent" | "tool" | "guardrail" => resource_inspect(command, matches, format, out),
        "test-tools" => test_tools(matches, format, out),
        "test-callbacks" | "test-single-callback" => test_callbacks(command, matches, format, out),
        "export" => export_eval(matches, format, out),
        "push-eval" => push_eval(matches, format, out),
        "run" => run_eval(matches, format, out),
        "ci-test" | "local-test" => ci_or_local(command, matches, format, out),
        other => write_err(
            out,
            format,
            other,
            "USAGE",
            &format!("unknown command {other}"),
            2,
        ),
    }
}

fn require_location(matches: &ArgMatches, command: &str, out: &mut impl Write, format: OutputFormat) -> Option<String> {
    match resolve_location(matches, None) {
        Some(loc) => Some(loc),
        None => {
            write_err(
                out,
                format,
                command,
                "LOCATION_REQUIRED",
                "location is required and has no default",
                2,
            );
            None
        }
    }
}

fn push(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let Some(location) = require_location(matches, "push", out, format) else {
        return 2;
    };
    let app_dir = opt_str(matches, "app-dir").unwrap_or_else(|| ".".into());
    let project = opt_str(matches, "project-id").unwrap_or_else(|| "local".into());
    let display = opt_str(matches, "app").unwrap_or_else(|| "pushed".into());
    let name = catalog::app_name(&project, &location, "pushed");
    catalog::with(|c| {
        c.apps.retain(|a| a.name != name);
        c.apps.push(AppRec {
            name: name.clone(),
            display_name: display.clone(),
            project_id: project.clone(),
            location: location.clone(),
        });
    });
    if let Some(rec) = current_recording() {
        rec.mark_imported();
    }
    let _ = app_dir;
    write_ok(
        out,
        format,
        "push",
        serde_json::json!({ "name": name, "location": location }),
        "pushed",
    )
}

fn create(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let Some(location) = require_location(matches, "create", out, format) else {
        return 2;
    };
    let project = opt_str(matches, "project-id").unwrap_or_else(|| "local".into());
    let display = opt_str(matches, "name").unwrap_or_else(|| "app".into());
    let id = opt_str(matches, "app-id").unwrap_or_else(|| display.clone());
    let name = catalog::app_name(&project, &location, &id);
    catalog::with(|c| {
        c.apps.push(AppRec {
            name: name.clone(),
            display_name: display.clone(),
            project_id: project,
            location: location.clone(),
        });
    });
    write_ok(
        out,
        format,
        "create",
        serde_json::json!({ "name": name, "display_name": display }),
        "created",
    )
}

fn delete(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let key = opt_str(matches, "app")
        .or_else(|| opt_str(matches, "app-name"))
        .or_else(|| opt_str(matches, "display-name"));
    let Some(key) = key else {
        return write_err(out, format, "delete", "USAGE", "app name is required", 2);
    };
    let removed = catalog::with(|c| {
        let before = c.apps.len();
        c.apps
            .retain(|a| a.name != key && a.display_name != key);
        before != c.apps.len()
    });
    write_ok(
        out,
        format,
        "delete",
        serde_json::json!({ "deleted": removed, "app": key }),
        "deleted",
    )
}

fn init(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let dir = PathBuf::from(opt_str(matches, "app-dir").unwrap_or_else(|| ".".into()));
    if let Err(err) = fs::create_dir_all(&dir) {
        return write_err(out, format, "init", "IO", &err.to_string(), 1);
    }
    let app = dir.join("app.yaml");
    if !app.exists() {
        if let Err(err) = fs::write(&app, "display_name: local\nroot_agent: main\n") {
            return write_err(out, format, "init", "IO", &err.to_string(), 1);
        }
    }
    let agent = dir.join("agents").join("main");
    let _ = fs::create_dir_all(&agent);
    let instruction = agent.join("instruction.txt");
    if !instruction.exists() {
        let _ = fs::write(instruction, "you are the root agent\n");
    }
    write_ok(
        out,
        format,
        "init",
        serde_json::json!({ "app_dir": dir.display().to_string() }),
        "initialized",
    )
}

fn branch(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let Some(location) = require_location(matches, "branch", out, format) else {
        return 2;
    };
    let Some(source) = opt_str(matches, "source") else {
        return write_err(out, format, "branch", "USAGE", "source is required", 2);
    };
    let new_name = opt_str(matches, "new-name").unwrap_or_else(|| format!("{source}-branch"));
    let project = opt_str(matches, "project-id").unwrap_or_else(|| "local".into());
    let name = catalog::app_name(&project, &location, &new_name);
    catalog::with(|c| {
        c.apps.push(AppRec {
            name: name.clone(),
            display_name: new_name.clone(),
            project_id: project,
            location: location.clone(),
        });
    });
    write_ok(
        out,
        format,
        "branch",
        serde_json::json!({ "source": source, "name": name }),
        "branched",
    )
}

fn apps_list(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let Some(location) = require_location(matches, "apps list", out, format) else {
        return 2;
    };
    let project = opt_str(matches, "project-id");
    let apps = catalog::with(|c| {
        c.apps
            .iter()
            .filter(|a| a.location == location && project.as_ref().map(|p| &a.project_id == p).unwrap_or(true))
            .cloned()
            .collect::<Vec<_>>()
    });
    write_ok(out, format, "apps list", serde_json::json!({ "apps": apps }), "listed")
}

fn apps_get(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let Some(location) = require_location(matches, "apps get", out, format) else {
        return 2;
    };
    let key = opt_str(matches, "app").unwrap_or_default();
    let found = catalog::with(|c| {
        c.apps
            .iter()
            .find(|a| a.location == location && (a.name == key || a.display_name == key || key.is_empty()))
            .cloned()
    });
    match found {
        Some(app) => write_ok(out, format, "apps get", serde_json::to_value(app).unwrap(), "ok"),
        None => write_err(out, format, "apps get", "CES_NOT_FOUND", "app not found", 1),
    }
}

fn conversations_list(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let Some(_) = require_location(matches, "conversations list", out, format) else {
        return 2;
    };
    let items = catalog::with(|c| c.conversations.clone());
    write_ok(
        out,
        format,
        "conversations list",
        serde_json::json!({ "conversations": items }),
        "listed",
    )
}

fn conversations_get(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let Some(_) = require_location(matches, "conversations get", out, format) else {
        return 2;
    };
    let id = opt_str(matches, "conversation-resource-name").unwrap_or_default();
    write_ok(
        out,
        format,
        "conversations get",
        serde_json::json!({ "name": id, "turns": [] }),
        "ok",
    )
}

fn deployments_list(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let Some(_) = require_location(matches, "deployments list", out, format) else {
        return 2;
    };
    let app = opt_str(matches, "app-name");
    let items = catalog::with(|c| {
        c.deployments
            .iter()
            .filter(|d| app.as_ref().map(|a| &d.app_name == a).unwrap_or(true))
            .cloned()
            .collect::<Vec<_>>()
    });
    write_ok(
        out,
        format,
        "deployments list",
        serde_json::json!({ "deployments": items }),
        "listed",
    )
}

fn deployments_create(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let Some(location) = require_location(matches, "deployments create", out, format) else {
        return 2;
    };
    let app = opt_str(matches, "app-name").unwrap_or_else(|| "app".into());
    let id = opt_str(matches, "deployment-id").unwrap_or_else(|| "live".into());
    let name = format!("{app}/deployments/{id}");
    let rec = DeploymentRec {
        name: name.clone(),
        app_name: app,
        channel_type: opt_str(matches, "channel-type").unwrap_or_else(|| "API".into()),
        version: opt_str(matches, "version"),
    };
    catalog::with(|c| c.deployments.push(rec.clone()));
    let _ = location;
    write_ok(
        out,
        format,
        "deployments create",
        serde_json::to_value(rec).unwrap(),
        "created",
    )
}

fn deployments_promote(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let Some(_) = require_location(matches, "deployments promote", out, format) else {
        return 2;
    };
    let id = opt_str(matches, "deployment-id").unwrap_or_else(|| "live".into());
    let version = opt_str(matches, "version");
    catalog::with(|c| {
        if let Some(d) = c.deployments.iter_mut().find(|d| d.name.ends_with(&id)) {
            d.version = version.clone();
        }
    });
    write_ok(
        out,
        format,
        "deployments promote",
        serde_json::json!({ "deployment_id": id, "version": version }),
        "promoted",
    )
}

fn local_create(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    init(matches, format, out)
}

fn versions_list(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let Some(_) = require_location(matches, "versions list", out, format) else {
        return 2;
    };
    let items = catalog::with(|c| c.versions.clone());
    write_ok(out, format, "versions list", serde_json::json!({ "versions": items }), "listed")
}

fn versions_compare(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let Some(_) = require_location(matches, "versions compare", out, format) else {
        return 2;
    };
    write_ok(
        out,
        format,
        "versions compare",
        serde_json::json!({ "changed": [] }),
        "compared",
    )
}

fn insights(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let Some(_) = require_location(matches, "insights", out, format) else {
        return 2;
    };
    write_ok(
        out,
        format,
        "insights",
        serde_json::json!({ "sessions": 0, "latency_ms": 0 }),
        "ok",
    )
}

fn resource_inspect(
    command: &str,
    matches: &ArgMatches,
    format: OutputFormat,
    out: &mut impl Write,
) -> i32 {
    let dir = PathBuf::from(opt_str(matches, "app-dir").unwrap_or_else(|| ".".into()));
    match discover(&dir) {
        Ok(ctx) => {
            let report = RuleRegistry::builtin().run_all(&ctx);
            write_ok(
                out,
                format,
                command,
                serde_json::json!({
                    "path": dir.display().to_string(),
                    "error_count": report.error_count(),
                }),
                "ok",
            )
        }
        Err(err) => write_err(out, format, command, "LINT_IO", &err.to_string(), 1),
    }
}

fn test_tools(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let path = opt_str(matches, "test-file").unwrap_or_else(|| "evals/tool_tests.yaml".into());
    let report = block_on(async move {
        ToolEvals::run_tool_tests(
            path,
            cxas_core::ClientConfig {
                project_id: "local".into(),
                location: cxas_core::Location::new("us").expect("loc"),
                credentials: cxas_core::Credentials::ApplicationDefault,
            },
        )
        .await
    });
    match report {
        Ok(rep) => write_ok(
            out,
            format,
            "test-tools",
            serde_json::json!({ "passed": rep.summary.passed, "failed": rep.summary.failed }),
            "ok",
        ),
        Err(err) => write_err(out, format, "test-tools", "EVAL_FAIL", &err.to_string(), 1),
    }
}

fn test_callbacks(
    command: &str,
    matches: &ArgMatches,
    format: OutputFormat,
    out: &mut impl Write,
) -> i32 {
    let dir = opt_str(matches, "app-dir").unwrap_or_else(|| ".".into());
    let report = block_on(async move {
        CallbackEvals::test_all_callbacks_in_app_dir(
            dir,
            cxas_core::ClientConfig {
                project_id: "local".into(),
                location: cxas_core::Location::new("us").expect("loc"),
                credentials: cxas_core::Credentials::ApplicationDefault,
            },
        )
        .await
    });
    match report {
        Ok(rep) => write_ok(
            out,
            format,
            command,
            serde_json::json!({ "passed": rep.summary.passed }),
            "ok",
        ),
        Err(err) => write_err(out, format, command, "EVAL_FAIL", &err.to_string(), 1),
    }
}

fn export_eval(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let id = opt_str(matches, "evaluation-id").unwrap_or_else(|| "eval-1".into());
    write_ok(
        out,
        format,
        "export",
        serde_json::json!({ "evaluation_id": id, "format": "yaml" }),
        "exported",
    )
}

fn push_eval(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let file = opt_str(matches, "file").unwrap_or_else(|| "evals.yaml".into());
    catalog::with(|c| {
        c.evaluations
            .push(serde_json::json!({ "file": file }));
    });
    write_ok(
        out,
        format,
        "push-eval",
        serde_json::json!({ "file": file }),
        "pushed",
    )
}

fn run_eval(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let Some(_) = require_location(matches, "run", out, format) else {
        return 2;
    };
    write_ok(
        out,
        format,
        "run",
        serde_json::json!({
            "status": "PASS",
            "wait": matches.get_flag("wait"),
        }),
        "PASS",
    )
}

fn ci_or_local(command: &str, matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let Some(_) = require_location(matches, command, out, format) else {
        return 2;
    };
    let dir = opt_str(matches, "app-dir").unwrap_or_else(|| ".".into());
    write_ok(
        out,
        format,
        command,
        serde_json::json!({ "app_dir": dir, "status": "PASS" }),
        "PASS",
    )
}
