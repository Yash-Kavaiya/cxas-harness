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

use bytes::Bytes;
use cxas_core::{
    AppName, Apps, CesTransport, ClientConfig, Credentials, ExportRequest, Location,
};
use futures::StreamExt;
use std::sync::Arc;

struct FiveMegMock;

#[async_trait::async_trait]
impl CesTransport for FiveMegMock {
    async fn export_app(
        &self,
        req: ExportRequest,
    ) -> Result<cxas_core::ExportHandle, cxas_core::CoreError> {
        assert_eq!(req.location, "us-central1");
        assert_eq!(req.version_id.as_deref(), Some("v3"));
        let chunk = Bytes::from(vec![7u8; 64 * 1024]);
        let chunks = std::iter::repeat(chunk).take(80); // 5 MiB
        Ok(cxas_core::ExportHandle::from_iter(chunks))
    }
}

#[tokio::test]
async fn export_app_version_streams_five_megabytes() {
    let cfg = ClientConfig {
        project_id: "p".into(),
        location: Location::new("us-central1").unwrap(),
        credentials: Credentials::ApplicationDefault,
    };
    let apps = Apps::new(cfg, Arc::new(FiveMegMock));
    let handle = apps
        .export_app_version(&AppName::parse("projects/p/locations/us-central1/apps/a").unwrap(), "v3")
        .await
        .unwrap();
    let mut total = 0usize;
    futures::pin_mut!(handle);
    while let Some(part) = handle.next().await {
        total += part.unwrap().len();
    }
    assert_eq!(total, 5 * 1024 * 1024);
}
