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

use cxas_state::{diff_trees, hash_app_dir, AppTree, StateError};
use std::fs;
use std::path::PathBuf;

fn write_tree(root: &std::path::Path, files: &[(&str, &str)]) {
    for (rel, body) in files {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }
}

#[test]
fn diff_reports_removed_tool() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a");
    let b = tmp.path().join("b");
    write_tree(
        &a,
        &[
            ("tools/alpha/tool.yaml", "x: 1"),
            ("tools/beta/tool.yaml", "y: 2"),
        ],
    );
    write_tree(&b, &[("tools/alpha/tool.yaml", "x: 1")]);
    let left = hash_app_dir(&a).unwrap();
    let right = hash_app_dir(&b).unwrap();
    let diff = diff_trees(&left, &right);
    assert!(diff
        .removed
        .iter()
        .any(|p| p == &PathBuf::from("tools/beta/tool.yaml")));
}

#[test]
fn identical_trees_have_equal_hashes() {
    let tmp = tempfile::tempdir().unwrap();
    write_tree(tmp.path(), &[("app.yaml", "display_name: d\n")]);
    let once = hash_app_dir(tmp.path()).unwrap();
    let twice = hash_app_dir(tmp.path()).unwrap();
    assert_eq!(once.root_hash, twice.root_hash);
}

#[test]
fn tools_symlink_outside_root_is_path_escape() {
    let tmp = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret"), "nope").unwrap();
    fs::create_dir_all(tmp.path().join("tools")).unwrap();
    let link = tmp.path().join("tools/evil");
    if link_dir_outside(outside.path(), &link).is_err() {
        return;
    }
    let err = hash_app_dir(tmp.path()).unwrap_err();
    assert!(matches!(err, StateError::PathEscape));
    let _empty = AppTree::empty();
}

fn link_dir_outside(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(src, dst)
    }
    #[cfg(windows)]
    {
        match std::os::windows::fs::symlink_dir(src, dst) {
            Ok(()) => Ok(()),
            Err(_) => {
                let status = std::process::Command::new("cmd")
                    .args([
                        "/C",
                        "mklink",
                        "/J",
                        &dst.to_string_lossy(),
                        &src.to_string_lossy(),
                    ])
                    .status()?;
                if status.success() {
                    Ok(())
                } else {
                    Err(std::io::Error::other("mklink /J failed"))
                }
            }
        }
    }
}
