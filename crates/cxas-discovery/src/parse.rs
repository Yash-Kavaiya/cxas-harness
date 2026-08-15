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

use crate::model::{Discovery, EnumField, Method};
use crate::DiscoveryError;
use serde_json::Value;
use std::path::Path;

impl Discovery {
    /// Parse a canonicalized discovery document from disk.
    ///
    /// Returns an error rather than an empty model on failure: an empty model
    /// would make every coverage and parity assertion pass vacuously, which is
    /// the exact self-grading failure this crate exists to prevent.
    pub fn load(path: &Path) -> Result<Self, DiscoveryError> {
        let text = std::fs::read_to_string(path).map_err(DiscoveryError::Io)?;
        Self::parse(&text)
    }

    /// Parse a discovery document already held in memory.
    pub fn parse(text: &str) -> Result<Self, DiscoveryError> {
        let root: Value = serde_json::from_str(text).map_err(DiscoveryError::Parse)?;

        let revision = root
            .get("revision")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let version = root
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        let mut methods = Vec::new();
        collect_methods(root.get("resources"), &mut methods);

        let mut enum_fields = Vec::new();
        if let Some(schemas) = root.get("schemas").and_then(Value::as_object) {
            for (schema_name, schema) in schemas {
                let Some(props) = schema.get("properties").and_then(Value::as_object) else {
                    continue;
                };
                for (prop_name, prop) in props {
                    let Some(values) = prop.get("enum").and_then(Value::as_array) else {
                        continue;
                    };
                    enum_fields.push(EnumField {
                        schema: schema_name.clone(),
                        property: prop_name.clone(),
                        values: values
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect(),
                    });
                }
            }
        }

        Ok(Discovery {
            revision,
            version,
            methods,
            enum_fields,
        })
    }
}

/// Discovery nests `resources` arbitrarily deep; every level may carry `methods`.
fn collect_methods(resources: Option<&Value>, out: &mut Vec<Method>) {
    let Some(map) = resources.and_then(Value::as_object) else {
        return;
    };
    for resource in map.values() {
        if let Some(methods) = resource.get("methods").and_then(Value::as_object) {
            for method in methods.values() {
                let (Some(id), Some(http_method), Some(path)) = (
                    method.get("id").and_then(Value::as_str),
                    method.get("httpMethod").and_then(Value::as_str),
                    method.get("path").and_then(Value::as_str),
                ) else {
                    continue;
                };
                out.push(Method {
                    id: id.to_string(),
                    http_method: http_method.to_string(),
                    path: path.to_string(),
                });
            }
        }
        collect_methods(resource.get("resources"), out);
    }
}
