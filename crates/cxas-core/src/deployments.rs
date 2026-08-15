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

use crate::transport::ChannelSettings;
use crate::{CesTransport, ClientConfig, CoreError};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeploymentName(String);

impl DeploymentName {
    pub fn parse(raw: impl AsRef<str>) -> Result<Self, CoreError> {
        let raw = raw.as_ref();
        let parts: Vec<&str> = raw.split('/').collect();
        if parts.len() == 8
            && parts[0] == "projects"
            && !parts[1].is_empty()
            && parts[2] == "locations"
            && !parts[3].is_empty()
            && parts[4] == "apps"
            && !parts[5].is_empty()
            && parts[6] == "deployments"
            && !parts[7].is_empty()
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deployment {
    pub name: String,
    pub channel_settings: ChannelSettings,
}

pub struct Deployments {
    #[allow(dead_code)]
    config: ClientConfig,
    transport: Arc<dyn CesTransport>,
}

impl Deployments {
    pub fn new(config: ClientConfig, transport: Arc<dyn CesTransport>) -> Self {
        Self { config, transport }
    }

    pub async fn update_channel_settings(
        &self,
        deployment: &DeploymentName,
        settings: ChannelSettings,
    ) -> Result<Deployment, CoreError> {
        let channel_settings = self
            .transport
            .update_channel_settings(deployment.as_str(), settings)
            .await?;
        Ok(Deployment {
            name: deployment.as_str().to_string(),
            channel_settings,
        })
    }
}
