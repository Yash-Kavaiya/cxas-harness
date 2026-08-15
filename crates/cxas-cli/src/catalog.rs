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

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppRec {
    pub name: String,
    pub display_name: String,
    pub project_id: String,
    pub location: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeploymentRec {
    pub name: String,
    pub app_name: String,
    pub channel_type: String,
    pub version: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Catalog {
    pub apps: Vec<AppRec>,
    pub deployments: Vec<DeploymentRec>,
    pub conversations: Vec<serde_json::Value>,
    pub versions: Vec<serde_json::Value>,
    pub evaluations: Vec<serde_json::Value>,
}

fn catalog_path() -> PathBuf {
    std::env::var_os("CXAS_CATALOG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".cxas/catalog.json"))
}

fn load() -> Catalog {
    let path = catalog_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save(catalog: &Catalog) {
    let path = catalog_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(catalog) {
        let _ = fs::write(path, raw);
    }
}

fn store() -> &'static Mutex<Catalog> {
    static STORE: OnceLock<Mutex<Catalog>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(load()))
}

pub fn with<T>(f: impl FnOnce(&mut Catalog) -> T) -> T {
    let mut guard = store().lock().expect("catalog");
    let result = f(&mut guard);
    save(&guard);
    result
}

pub fn app_name(project: &str, location: &str, id: &str) -> String {
    format!("projects/{project}/locations/{location}/apps/{id}")
}
