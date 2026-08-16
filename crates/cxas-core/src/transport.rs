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
use std::sync::Mutex;
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
    // Named for what it does, not for the trait it resembles. `FromIterator`
    // cannot be implemented here: the handle borrows nothing and yields chunks
    // lazily, so there is no collection to build.
    #[allow(clippy::should_implement_trait)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelSettings {
    pub noise_cancellation: Option<bool>,
    pub noise_suppression_level: Option<u32>,
}

#[async_trait::async_trait]
pub trait CesTransport: Send + Sync {
    async fn export_app(&self, req: ExportRequest) -> Result<ExportHandle, CoreError>;

    async fn update_channel_settings(
        &self,
        _name: &str,
        _settings: ChannelSettings,
    ) -> Result<ChannelSettings, CoreError> {
        Err(CoreError::Transport(
            "update_channel_settings not implemented".into(),
        ))
    }
}

pub struct NoopTransport;

#[async_trait::async_trait]
impl CesTransport for NoopTransport {
    async fn export_app(&self, _req: ExportRequest) -> Result<ExportHandle, CoreError> {
        Ok(ExportHandle::from_iter(std::iter::empty()))
    }
}

#[derive(Default)]
pub struct RecordingTransport {
    last_channel_settings: Mutex<Option<ChannelSettings>>,
}

impl RecordingTransport {
    pub fn last_channel_settings(&self) -> Option<ChannelSettings> {
        *self.last_channel_settings.lock().expect("lock")
    }
}

#[async_trait::async_trait]
impl CesTransport for RecordingTransport {
    async fn export_app(&self, _req: ExportRequest) -> Result<ExportHandle, CoreError> {
        Ok(ExportHandle::from_iter(std::iter::empty()))
    }

    async fn update_channel_settings(
        &self,
        _name: &str,
        settings: ChannelSettings,
    ) -> Result<ChannelSettings, CoreError> {
        *self.last_channel_settings.lock().expect("lock") = Some(settings);
        Ok(settings)
    }
}
