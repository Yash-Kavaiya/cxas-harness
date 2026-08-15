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

use crate::{CesTransport, ClientConfig, CoreError, ExportHandle, ExportRequest};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AppName(String);

impl AppName {
    pub fn parse(raw: impl AsRef<str>) -> Result<Self, CoreError> {
        let raw = raw.as_ref();
        let parts: Vec<&str> = raw.split('/').collect();
        if parts.len() == 6
            && parts[0] == "projects"
            && !parts[1].is_empty()
            && parts[2] == "locations"
            && !parts[3].is_empty()
            && parts[4] == "apps"
            && !parts[5].is_empty()
        {
            Ok(Self(raw.to_string()))
        } else {
            Err(CoreError::InvalidName(raw.to_string()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub struct Apps {
    config: ClientConfig,
    transport: Arc<dyn CesTransport>,
}

impl Apps {
    pub fn new(config: ClientConfig, transport: Arc<dyn CesTransport>) -> Self {
        Self { config, transport }
    }

    pub async fn export_app(&self, name: &AppName) -> Result<ExportHandle, CoreError> {
        self.transport
            .export_app(self.export_request(name, None))
            .await
    }

    pub async fn export_app_version(
        &self,
        name: &AppName,
        version_id: &str,
    ) -> Result<ExportHandle, CoreError> {
        self.transport
            .export_app(self.export_request(name, Some(version_id.to_string())))
            .await
    }

    fn export_request(&self, name: &AppName, version_id: Option<String>) -> ExportRequest {
        ExportRequest {
            location: self.config.location.as_str().to_string(),
            name: name.as_str().to_string(),
            version_id,
        }
    }
}
