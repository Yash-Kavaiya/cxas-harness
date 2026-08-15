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

use cxas_lint::{discover, schema_map, RuleRegistry};

#[test]
fn registry_has_at_least_sixty_rules() {
    assert!(RuleRegistry::builtin().ids().len() >= 60);
}

#[test]
fn every_required_field_has_a_failing_fixture() {
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../../../schema/app.required.json")).unwrap();
    for (section, fields) in schema.as_object().unwrap() {
        for field in fields.as_array().unwrap() {
            let field = field.as_str().unwrap();
            let rule_id = schema_map::rule_id_for(section, field)
                .unwrap_or_else(|| panic!("no rule mapped for {section}.{field}"));
            let dir = schema_map::fixture_omitting(section, field);
            let ctx = discover(dir.path()).unwrap();
            let report = RuleRegistry::builtin().run_all(&ctx);
            assert!(
                report.diagnostics.iter().any(|d| d.rule_id == rule_id && d.severity == cxas_lint::Severity::Error),
                "{section}.{field} should trigger {rule_id}"
            );
        }
    }
}
