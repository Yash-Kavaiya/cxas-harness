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

use cxas_migration::ToolSync;
use cxas_state::{hash_app_dir, AppTree};
use std::fs;
use std::path::PathBuf;

#[tokio::test]
async fn deletes_local_tool_missing_from_remote() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir_all(root.join("tools/alpha")).unwrap();
    fs::write(root.join("tools/alpha/tool.yaml"), "name: alpha\n").unwrap();
    fs::create_dir_all(root.join("tools/beta")).unwrap();
    fs::write(root.join("tools/beta/tool.yaml"), "name: beta\n").unwrap();

    let remote_dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(remote_dir.path().join("tools/alpha")).unwrap();
    fs::write(
        remote_dir.path().join("tools/alpha/tool.yaml"),
        "name: alpha\n",
    )
    .unwrap();
    let remote = hash_app_dir(remote_dir.path()).unwrap();

    let report = ToolSync::new().reconcile(root, &remote).await.unwrap();
    assert!(!root.join("tools/beta").exists());
    assert!(root.join("tools/alpha").exists());
    assert!(report
        .deleted_local
        .iter()
        .any(|p| p == &PathBuf::from("tools/beta/tool.yaml") || p.starts_with("tools/beta")));
}

#[tokio::test]
async fn refuses_to_follow_symlink_outside_root() {
    let tmp = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret"), "nope").unwrap();
    fs::create_dir_all(tmp.path().join("tools")).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.path(), tmp.path().join("tools/evil")).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(outside.path(), tmp.path().join("tools/evil")).unwrap();

    let remote = AppTree::empty();
    let err = ToolSync::new()
        .reconcile(tmp.path(), &remote)
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        cxas_migration::LifeError::State(cxas_state::StateError::PathEscape)
    ));
    assert!(outside.path().join("secret").exists());
}
