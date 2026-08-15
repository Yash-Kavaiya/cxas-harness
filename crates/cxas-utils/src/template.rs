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

use crate::UtilsError;
use serde_json::{Map, Number, Value};
use std::collections::BTreeMap;

/// Variable value usable in environment templates.
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateValue {
    String(String),
    Bool(bool),
    Number(Number),
}

/// Render `template` by substituting `{{NAME}}` / `{{NAME|bool}}` placeholders.
///
/// When a JSON string is exactly `{{NAME}}` and the variable is a `Bool` or
/// `Number`, the placeholder is replaced with a typed JSON value (not a string).
/// `{{NAME|bool}}` coerces a string variable of `"true"` / `"false"` only.
pub fn render_environment(
    template: &Value,
    vars: &BTreeMap<String, TemplateValue>,
) -> Result<Value, UtilsError> {
    render_value(template, vars)
}

fn render_value(
    value: &Value,
    vars: &BTreeMap<String, TemplateValue>,
) -> Result<Value, UtilsError> {
    match value {
        Value::Object(map) => {
            let mut out = Map::with_capacity(map.len());
            for (k, v) in map {
                out.insert(k.clone(), render_value(v, vars)?);
            }
            Ok(Value::Object(out))
        }
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(render_value(item, vars)?);
            }
            Ok(Value::Array(out))
        }
        Value::String(s) => render_string(s, vars),
        other => Ok(other.clone()),
    }
}

fn render_string(
    s: &str,
    vars: &BTreeMap<String, TemplateValue>,
) -> Result<Value, UtilsError> {
    if let Some(name) = exact_placeholder(s) {
        return match vars.get(name) {
            Some(TemplateValue::Bool(b)) => Ok(Value::Bool(*b)),
            Some(TemplateValue::Number(n)) => Ok(Value::Number(n.clone())),
            Some(TemplateValue::String(v)) => Ok(Value::String(v.clone())),
            None => Ok(Value::String(s.to_string())),
        };
    }

    if let Some(name) = exact_bool_placeholder(s) {
        return match vars.get(name) {
            Some(TemplateValue::Bool(b)) => Ok(Value::Bool(*b)),
            Some(TemplateValue::String(v)) => match v.as_str() {
                "true" => Ok(Value::Bool(true)),
                "false" => Ok(Value::Bool(false)),
                _ => Err(UtilsError::InvalidBoolTemplate),
            },
            Some(TemplateValue::Number(_)) | None => Err(UtilsError::InvalidBoolTemplate),
        };
    }

    Ok(Value::String(s.to_string()))
}

/// Returns `Some(name)` when `s` is exactly `{{name}}` (no filter).
fn exact_placeholder(s: &str) -> Option<&str> {
    let inner = s.strip_prefix("{{")?.strip_suffix("}}")?;
    if inner.is_empty() || inner.contains('|') || inner.contains('{') || inner.contains('}') {
        return None;
    }
    Some(inner)
}

/// Returns `Some(name)` when `s` is exactly `{{name|bool}}`.
fn exact_bool_placeholder(s: &str) -> Option<&str> {
    let inner = s.strip_prefix("{{")?.strip_suffix("}}")?;
    let (name, filter) = inner.split_once('|')?;
    if filter != "bool" || name.is_empty() || name.contains('{') || name.contains('}') {
        return None;
    }
    Some(name)
}
