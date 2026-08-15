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
use cxas_core::{CesTransport, CoreError, ExportHandle, ExportRequest, NoopTransport};
use cxas_state::{hash_bytes, AppTree};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Inner {
    export_stub: Vec<u8>,
    last_export_version: Option<String>,
    last_export_bytes: usize,
    imported: bool,
    version_created: bool,
    deployment_created: bool,
    remote_tree: Option<AppTree>,
}

#[derive(Clone, Default)]
pub struct RecordingTransport {
    inner: Arc<Mutex<Inner>>,
}

impl RecordingTransport {
    pub fn stub_export(&self, bytes: Vec<u8>) {
        self.inner.lock().expect("lock").export_stub = bytes;
    }

    pub fn last_export_version(&self) -> Option<String> {
        self.inner.lock().expect("lock").last_export_version.clone()
    }

    pub fn last_export_bytes(&self) -> usize {
        self.inner.lock().expect("lock").last_export_bytes
    }

    pub fn imported(&self) -> bool {
        self.inner.lock().expect("lock").imported
    }

    pub fn version_created(&self) -> bool {
        self.inner.lock().expect("lock").version_created
    }

    pub fn deployment_created(&self) -> bool {
        self.inner.lock().expect("lock").deployment_created
    }

    pub fn stub_remote_tree(&self, files: &[(&str, &str)]) {
        let mut tree = AppTree::empty();
        for (path, content) in files {
            tree.files
                .insert(PathBuf::from(*path), hash_bytes(content.as_bytes()));
        }
        self.inner.lock().expect("lock").remote_tree = Some(tree);
    }

    pub fn mark_imported(&self) {
        self.inner.lock().expect("lock").imported = true;
    }

    pub fn mark_version_created(&self) {
        self.inner.lock().expect("lock").version_created = true;
    }

    pub fn mark_deployment_created(&self) {
        self.inner.lock().expect("lock").deployment_created = true;
    }

    pub fn remote_tree(&self) -> Option<AppTree> {
        self.inner.lock().expect("lock").remote_tree.clone()
    }
}

#[async_trait::async_trait]
impl CesTransport for RecordingTransport {
    async fn export_app(&self, req: ExportRequest) -> Result<ExportHandle, CoreError> {
        let mut inner = self.inner.lock().expect("lock");
        inner.last_export_version = req.version_id.clone();
        inner.last_export_bytes = inner.export_stub.len();
        let bytes = inner.export_stub.clone();
        drop(inner);
        Ok(ExportHandle::from_iter([Bytes::from(bytes)]))
    }
}

static TEST_TRANSPORT: Mutex<Option<RecordingTransport>> = Mutex::new(None);
static SCRIPT_TRACE: Mutex<Option<Vec<serde_json::Value>>> = Mutex::new(None);

pub fn set_transport_for_test(transport: RecordingTransport) {
    *TEST_TRANSPORT.lock().expect("lock") = Some(transport);
}

pub fn current_recording() -> Option<RecordingTransport> {
    TEST_TRANSPORT.lock().expect("lock").clone()
}

pub fn ces_transport() -> Arc<dyn CesTransport> {
    match current_recording() {
        Some(rec) => Arc::new(rec),
        None => Arc::new(NoopTransport),
    }
}

pub fn script_trace(turns: Vec<serde_json::Value>) {
    *SCRIPT_TRACE.lock().expect("lock") = Some(turns);
}

pub fn take_scripted_trace() -> Vec<serde_json::Value> {
    SCRIPT_TRACE.lock().expect("lock").take().unwrap_or_default()
}

pub fn block_on<T, F>(fut: F) -> T
where
    T: Send + 'static,
    F: std::future::Future<Output = T> + Send + 'static,
{
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime")
            .block_on(fut)
    })
    .join()
    .expect("runtime thread")
}
