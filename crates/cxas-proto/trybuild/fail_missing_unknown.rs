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

fn label(s: cxas_proto::EvaluationRunState) -> &'static str {
    match s {
        cxas_proto::EvaluationRunState::Unspecified => "u",
        cxas_proto::EvaluationRunState::Queued => "q",
        cxas_proto::EvaluationRunState::Running => "r",
        cxas_proto::EvaluationRunState::Completed => "d",
        cxas_proto::EvaluationRunState::Error => "e",
        cxas_proto::EvaluationRunState::Cancelled => "c",
    }
}

fn main() {
    let _ = label(cxas_proto::EvaluationRunState::Queued);
}
