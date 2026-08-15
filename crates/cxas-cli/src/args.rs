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
        .disable_help_subcommand(false)
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
        .subcommand(lint_cmd())
        .subcommand(pull_cmd())
        .subcommand(actions_cmd())
        .subcommand(init_github_action_cmd())
        .subcommand(trace_cmd())
        .subcommand(evals_cmd())
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
