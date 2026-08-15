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
}

/// A parsed discovery document.
#[derive(Debug, Clone, Default)]
pub struct Discovery {
    pub(crate) revision: String,
    pub(crate) version: String,
    pub(crate) methods: Vec<Method>,
    pub(crate) enum_fields: Vec<EnumField>,
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
}
