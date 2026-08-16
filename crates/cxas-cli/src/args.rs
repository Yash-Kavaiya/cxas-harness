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

use clap::{Arg, ArgAction, Command};

pub fn build_parser() -> Command {
    Command::new("cxas")
        .about("Machine-first CXAS CLI")
        .no_binary_name(false)
        .disable_help_subcommand(true)
        .arg(
            Arg::new("format")
                .long("format")
                .global(true)
                .default_value("json")
                .value_parser(["json", "human"]),
        )
        .arg(
            Arg::new("no-input")
                .long("no-input")
                .global(true)
                .action(ArgAction::SetTrue)
                .default_value("true")
                .default_missing_value("true"),
        )
        .arg(
            Arg::new("oauth-token")
                .long("oauth-token")
                .global(true)
                .num_args(1),
        )
        .subcommand(api_cmd())
        .subcommand(lint_cmd())
        .subcommand(pull_cmd())
        .subcommand(actions_cmd())
        .subcommand(init_github_action_cmd())
        .subcommand(trace_cmd())
        .subcommand(evals_cmd())
        .subcommand(deploy_cmd())
        .subcommand(diff_cmd())
        .subcommand(state_cmd())
        .subcommand(migrate_cmd())
        .subcommand(file_cmd("test-tools", "test-file"))
        .subcommand(dir_cmd("test-callbacks"))
        .subcommand(dir_cmd("test-single-callback"))
        .subcommand(eval_file_cmd("export", "evaluation-id"))
        .subcommand(eval_file_cmd("push-eval", "file"))
        .subcommand(run_cmd())
        .subcommand(leaf("run-session"))
        .subcommand(ci_cmd("ci-test"))
        .subcommand(ci_cmd("local-test"))
        .subcommand(delete_cmd())
        .subcommand(push_cmd())
        .subcommand(dir_cmd("llm-lint"))
        .subcommand(Command::new("help"))
        .subcommand(dir_cmd("init"))
        .subcommand(create_cmd())
        .subcommand(branch_cmd())
        .subcommand(apps_cmd())
        .subcommand(conversations_cmd())
        .subcommand(deployments_cmd())
        .subcommand(parent_dir("local", "create"))
        .subcommand(versions_cmd())
        .subcommand(located("insights"))
        .subcommand(dir_cmd("agent"))
        .subcommand(dir_cmd("tool"))
        .subcommand(dir_cmd("guardrail"))
}

/// `cxas api` -- list, describe, and issue any declared CES method.
///
/// Generic on purpose: 170 methods do not need 170 subcommands, and the ones
/// worth naming already have their own verbs elsewhere in this binary.
fn api_cmd() -> Command {
    Command::new("api")
        .about("Address the CES REST surface directly")
        .subcommand(
            Command::new("list")
                .about("List every method CES declares")
                .arg(api_version_arg())
                .arg(Arg::new("filter").long("filter").num_args(1))
                .arg(
                    Arg::new("modelled")
                        .long("modelled")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("describe")
                .about("Show one method's verb, path, and parameters")
                .arg(Arg::new("method").num_args(1))
                .arg(api_version_arg()),
        )
        .subcommand(api_request_cmd("call", "Issue a method and print its response"))
        .subcommand(api_request_cmd(
            "stream",
            "Issue a streaming method, printing each message as it arrives",
        ))
}

fn api_version_arg() -> Arg {
    Arg::new("api-version")
        .long("api-version")
        .num_args(1)
        .value_parser(["v1", "v1beta"])
}

fn api_request_cmd(name: &'static str, about: &'static str) -> Command {
    Command::new(name)
        .about(about)
        .arg(Arg::new("method").num_args(1))
        .arg(api_version_arg())
        .arg(
            Arg::new("param")
                .long("param")
                .short('p')
                .num_args(1)
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("query")
                .long("query")
                .short('q')
                .num_args(1)
                .action(ArgAction::Append),
        )
        .arg(Arg::new("body").long("body").num_args(1))
        .arg(Arg::new("endpoint").long("endpoint").num_args(1))
}

fn loc_args() -> [Arg; 2] {
    [
        Arg::new("location").long("location").num_args(1),
        Arg::new("project-id").long("project-id").num_args(1),
    ]
}

fn leaf(name: &'static str) -> Command {
    Command::new(name)
}

fn located(name: &'static str) -> Command {
    Command::new(name).args(loc_args())
}

fn dir_cmd(name: &'static str) -> Command {
    Command::new(name)
        .arg(Arg::new("app-dir").long("app-dir").num_args(1))
        .args(loc_args())
}

fn file_cmd(name: &'static str, file_flag: &'static str) -> Command {
    Command::new(name)
        .arg(Arg::new(file_flag).long(file_flag).num_args(1))
        .arg(Arg::new("app-name").long("app-name").num_args(1))
        .args(loc_args())
}

fn eval_file_cmd(name: &'static str, file_flag: &'static str) -> Command {
    Command::new(name)
        .arg(Arg::new(file_flag).long(file_flag).num_args(1))
        .arg(Arg::new("app-name").long("app-name").num_args(1))
        .arg(Arg::new("output").long("output").num_args(1))
        .args(loc_args())
}

fn ci_cmd(name: &'static str) -> Command {
    Command::new(name)
        .arg(Arg::new("app-dir").long("app-dir").num_args(1))
        .arg(Arg::new("display-name").long("display-name").num_args(1))
        .args(loc_args())
}

fn create_cmd() -> Command {
    Command::new("create")
        .arg(Arg::new("name").long("name").num_args(1))
        .arg(Arg::new("app-id").long("app-id").num_args(1))
        .args(loc_args())
}

fn delete_cmd() -> Command {
    Command::new("delete")
        .arg(Arg::new("app").long("app").num_args(1))
        .arg(Arg::new("app-name").long("app-name").num_args(1))
        .arg(Arg::new("display-name").long("display-name").num_args(1))
        .args(loc_args())
}

fn branch_cmd() -> Command {
    Command::new("branch")
        .arg(Arg::new("source").long("source").num_args(1))
        .arg(Arg::new("new-name").long("new-name").num_args(1))
        .args(loc_args())
}

fn apps_cmd() -> Command {
    Command::new("apps")
        .subcommand(
            Command::new("list")
                .args(loc_args())
                .arg(Arg::new("app").long("app").num_args(1)),
        )
        .subcommand(
            Command::new("get")
                .args(loc_args())
                .arg(Arg::new("app").long("app").num_args(1)),
        )
}

fn conversations_cmd() -> Command {
    Command::new("conversations")
        .subcommand(
            Command::new("list")
                .args(loc_args())
                .arg(Arg::new("app-name").long("app-name").num_args(1)),
        )
        .subcommand(
            Command::new("get")
                .args(loc_args())
                .arg(
                    Arg::new("conversation-resource-name")
                        .long("conversation-resource-name")
                        .num_args(1),
                ),
        )
}

fn deployments_cmd() -> Command {
    let common = || {
        [
            Arg::new("app-name").long("app-name").num_args(1),
            Arg::new("deployment-id").long("deployment-id").num_args(1),
            Arg::new("version").long("version").num_args(1),
            Arg::new("channel-type").long("channel-type").num_args(1),
        ]
    };
    Command::new("deployments")
        .subcommand(Command::new("list").args(loc_args()).args(common()))
        .subcommand(Command::new("create").args(loc_args()).args(common()))
        .subcommand(Command::new("promote").args(loc_args()).args(common()))
}

fn versions_cmd() -> Command {
    Command::new("versions")
        .subcommand(
            Command::new("list")
                .args(loc_args())
                .arg(Arg::new("app").long("app").num_args(1)),
        )
        .subcommand(
            Command::new("compare")
                .args(loc_args())
                .arg(Arg::new("app").long("app").num_args(1)),
        )
}

fn parent_dir(name: &'static str, child: &'static str) -> Command {
    Command::new(name).subcommand(dir_cmd(child))
}

fn migrate_cmd() -> Command {
    Command::new("migrate").subcommand(
        Command::new("dfcx")
            .arg(Arg::new("source").long("source").num_args(1))
            .arg(Arg::new("agent-id").long("agent-id").num_args(1))
            .arg(Arg::new("zip").long("zip").num_args(1))
            .arg(Arg::new("project-id").long("project-id").num_args(1))
            .arg(Arg::new("location").long("location").num_args(1))
            .arg(Arg::new("target-name").long("target-name").num_args(1))
            .arg(Arg::new("display-name").long("display-name").num_args(1))
            .arg(Arg::new("profile").long("profile").num_args(1))
            .arg(Arg::new("yes").long("yes").action(ArgAction::SetTrue)),
    )
}

fn run_cmd() -> Command {
    Command::new("run")
        .arg(Arg::new("wait").long("wait").action(ArgAction::SetTrue))
        .arg(Arg::new("app-dir").long("app-dir").num_args(1))
        .arg(Arg::new("app-name").long("app-name").num_args(1))
        .arg(Arg::new("evaluation-id").long("evaluation-id").num_args(1))
        .args(loc_args())
}

fn push_cmd() -> Command {
    Command::new("push")
        .arg(Arg::new("app-dir").long("app-dir").num_args(1))
        .arg(Arg::new("app").long("app").num_args(1))
        .args(loc_args())
}

fn lint_cmd() -> Command {
    Command::new("lint").arg(
        Arg::new("app-dir")
            .long("app-dir")
            .num_args(1)
            .default_value("."),
    )
}

fn pull_cmd() -> Command {
    Command::new("pull")
        .arg(Arg::new("app").long("app").num_args(1).required(true))
        .arg(
            Arg::new("target-dir")
                .long("target-dir")
                .num_args(1)
                .required(true),
        )
        .arg(Arg::new("location").long("location").num_args(1))
        .arg(Arg::new("version-id").long("version-id").num_args(1))
        .arg(Arg::new("project-id").long("project-id").num_args(1))
}

fn actions_flags() -> impl Iterator<Item = Arg> {
    [
        Arg::new("app-dir")
            .long("app-dir")
            .num_args(1)
            .default_value("."),
        Arg::new("auto-create-wif")
            .long("auto-create-wif")
            .action(ArgAction::SetTrue),
        Arg::new("no-cleanup")
            .long("no-cleanup")
            .action(ArgAction::SetTrue),
        Arg::new("workload-identity-provider")
            .long("workload-identity-provider")
            .num_args(1),
        Arg::new("service-account")
            .long("service-account")
            .num_args(1),
    ]
    .into_iter()
}

fn actions_cmd() -> Command {
    Command::new("actions").subcommand(Command::new("init").args(actions_flags()))
}

fn init_github_action_cmd() -> Command {
    Command::new("init-github-action").args(actions_flags())
}

fn trace_cmd() -> Command {
    Command::new("trace")
        .arg(Arg::new("app-name").long("app-name").num_args(1))
        .arg(Arg::new("app").long("app").num_args(1))
        .arg(Arg::new("location").long("location").num_args(1))
        .arg(Arg::new("raw").long("raw").action(ArgAction::SetTrue))
}

fn evals_cmd() -> Command {
    Command::new("evals").subcommand(
        Command::new("report")
            .arg(Arg::new("output-dir").long("output-dir").num_args(1))
            .arg(Arg::new("output").long("output").num_args(1)),
    )
}

fn deploy_cmd() -> Command {
    Command::new("deploy")
        .arg(
            Arg::new("app-dir")
                .long("app-dir")
                .num_args(1)
                .default_value("."),
        )
        .arg(Arg::new("project-id").long("project-id").num_args(1))
        .arg(Arg::new("location").long("location").num_args(1))
        .arg(Arg::new("channel-type").long("channel-type").num_args(1))
        .arg(
            Arg::new("noise-cancellation")
                .long("noise-cancellation")
                .action(ArgAction::SetTrue),
        )
}

fn diff_cmd() -> Command {
    Command::new("diff")
        .arg(
            Arg::new("app-dir")
                .long("app-dir")
                .num_args(1)
                .default_value("."),
        )
        .arg(Arg::new("location").long("location").num_args(1))
        .arg(Arg::new("app").long("app").num_args(1))
        .arg(
            Arg::new("allow-drift")
                .long("allow-drift")
                .action(ArgAction::SetTrue),
        )
}

fn state_cmd() -> Command {
    Command::new("state")
        .arg(
            Arg::new("app-dir")
                .long("app-dir")
                .num_args(1)
                .default_value("."),
        )
        .arg(Arg::new("location").long("location").num_args(1))
        .arg(Arg::new("project-id").long("project-id").num_args(1))
}
