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

use crate::CoreError;
use bytes::Bytes;
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct ExportRequest {
    pub location: String,
    pub name: String,
    pub version_id: Option<String>,
}

pub struct ExportHandle {
    inner: Pin<Box<dyn Stream<Item = Result<Bytes, CoreError>> + Send>>,
}

impl ExportHandle {
    pub fn from_iter<I>(chunks: I) -> Self
    where
        I: IntoIterator<Item = Bytes>,
        I::IntoIter: Send + 'static,
    {
        Self {
            inner: Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))),
        }
    }
}

impl Stream for ExportHandle {
    type Item = Result<Bytes, CoreError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

#[async_trait::async_trait]
pub trait CesTransport: Send + Sync {
    async fn export_app(&self, req: ExportRequest) -> Result<ExportHandle, CoreError>;
}
