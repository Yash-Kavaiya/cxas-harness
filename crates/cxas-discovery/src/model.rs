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

/// One REST method from a discovery document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Method {
    pub id: String,
    pub http_method: String,
    pub path: String,
}

/// One enum-valued property on a schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumField {
    pub schema: String,
    pub property: String,
    pub values: Vec<String>,
    /// True when discovery declares the property as an array of these values,
    /// i.e. the wire type is a list, not a single value.
    pub repeated: bool,
}

/// One enum-valued query parameter on a REST method.
///
/// Kept apart from [`EnumField`] because a parameter is not a schema property:
/// it has no schema behind it, so a schema-keyed lookup could never find it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterEnum {
    pub method_id: String,
    pub parameter: String,
    pub values: Vec<String>,
    /// True when discovery marks the parameter `repeated`, i.e. it may be
    /// supplied more than once in the query string.
    pub repeated: bool,
}

/// A parsed discovery document.
#[derive(Debug, Clone, Default)]
pub struct Discovery {
    pub(crate) revision: String,
    pub(crate) version: String,
    pub(crate) methods: Vec<Method>,
    pub(crate) enum_fields: Vec<EnumField>,
    pub(crate) parameter_enums: Vec<ParameterEnum>,
}

impl Discovery {
    /// Upstream revision stamp, used to detect drift against the pinned copy.
    pub fn revision(&self) -> &str {
        &self.revision
    }

    /// API version this document describes, e.g. `v1beta`.
    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn methods(&self) -> impl Iterator<Item = &Method> {
        self.methods.iter()
    }

    pub fn method(&self, id: &str) -> Option<&Method> {
        self.methods.iter().find(|m| m.id == id)
    }

    pub fn enum_fields(&self) -> impl Iterator<Item = &EnumField> {
        self.enum_fields.iter()
    }

    pub fn enum_field(&self, schema: &str, property: &str) -> Option<&EnumField> {
        self.enum_fields
            .iter()
            .find(|e| e.schema == schema && e.property == property)
    }

    pub fn parameter_enums(&self) -> impl Iterator<Item = &ParameterEnum> {
        self.parameter_enums.iter()
    }

    pub fn parameter_enum(&self, method_id: &str, parameter: &str) -> Option<&ParameterEnum> {
        self.parameter_enums
            .iter()
            .find(|p| p.method_id == method_id && p.parameter == parameter)
    }
}
