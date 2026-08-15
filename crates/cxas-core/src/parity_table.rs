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

/// Phase 0 Python classes owned by `cxas-core` (`rust_owner == "cxas-core"`).
pub const CORE_PYTHON_CLASSES: &[&str] = &[
    stringify!(Agents),
    stringify!(Apps),
    stringify!(Callbacks),
    stringify!(Changelogs),
    stringify!(Common),
    stringify!(ConversationHistory),
    stringify!(Deployments),
    stringify!(Evaluations),
    stringify!(Guardrails),
    stringify!(Sessions),
    stringify!(Tools),
    stringify!(Variables),
    stringify!(Versions),
];
