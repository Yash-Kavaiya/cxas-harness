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

#[test]
fn every_parity_command_is_a_clap_subcommand() {
    let manifest = cxas_parity::load_bundled().unwrap();
    let parser = cxas_cli::build_parser();
    for cmd in manifest.commands_for_crate("cxas-cli") {
        let mut current = &parser;
        for (i, part) in cmd.argv.iter().enumerate() {
            current = current
                .find_subcommand(part)
                .unwrap_or_else(|| panic!("missing clap path {:?} at {part}", cmd.argv));
            if i + 1 == cmd.argv.len() {
                break;
            }
        }
    }
}
