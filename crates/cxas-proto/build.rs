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

//! Compiles `proto/ces/evaluation_run_state.proto` when `protoc` is available.
//! The public `EvaluationRunState` wrapper is hand-written; generation is kept
//! so later protos have a build path. Without `protoc`, this is a no-op.

fn main() {
    let proto = "../../proto/ces/evaluation_run_state.proto";
    let include = "../../proto";
    println!("cargo:rerun-if-changed={proto}");

    let protoc_ok = std::process::Command::new("protoc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !protoc_ok {
        println!(
            "cargo:warning=protoc not found; skipping proto codegen \
             (hand-written EvaluationRunState remains the public API)"
        );
        return;
    }

    tonic_build::configure()
        .build_server(false)
        .build_client(false)
        .compile_protos(&[proto], &[include])
        .expect("failed to compile evaluation_run_state.proto");
}
