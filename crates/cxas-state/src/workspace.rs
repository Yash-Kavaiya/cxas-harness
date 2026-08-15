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

use crate::StateError;
use cxas_core::{CoreError, Location};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Cascading workspace profile after `extends` overlay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceProfile {
    pub name: String,
    pub project_id: String,
    pub location: Location,
    pub extends: Option<String>,
}

/// Resolved workspace; same fields as [`WorkspaceProfile`].
pub type ResolvedWorkspace = WorkspaceProfile;

#[derive(Debug, Deserialize)]
struct WorkspaceFile {
    #[serde(default)]
    profiles: BTreeMap<String, RawProfile>,
    #[serde(default)]
    active: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct RawProfile {
    #[serde(default)]
    project_id: Option<String>,
    #[serde(default)]
    location: Option<String>,
    #[serde(default)]
    extends: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UserProfilesFile {
    #[serde(default)]
    profiles: Option<BTreeMap<String, RawProfile>>,
}

/// Walk `cwd` → parents for `cxas.workspace.yaml`, then apply cascading profiles.
pub fn resolve_workspace(cwd: &Path) -> Result<ResolvedWorkspace, StateError> {
    let file = find_workspace_file(cwd)?;
    let text = std::fs::read_to_string(&file)?;
    let parsed: WorkspaceFile = serde_yaml::from_str(&text)?;
    let active = parsed
        .active
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(StateError::ActiveProfileMissing)?;

    let user = load_user_profiles();
    let mut stack = Vec::new();
    let merged = resolve_profile(active, &parsed.profiles, &user, &mut stack)?;
    let location = location_from(merged.location.as_deref())?;

    Ok(WorkspaceProfile {
        name: active.to_string(),
        project_id: merged.project_id.unwrap_or_default(),
        location,
        extends: parsed
            .profiles
            .get(active)
            .and_then(|p| p.extends.clone())
            .or(merged.extends),
    })
}

fn find_workspace_file(cwd: &Path) -> Result<PathBuf, StateError> {
    let mut dir = if cwd.as_os_str().is_empty() {
        std::env::current_dir()?
    } else if cwd.is_absolute() {
        cwd.to_path_buf()
    } else {
        std::env::current_dir()?.join(cwd)
    };

    loop {
        let candidate = dir.join("cxas.workspace.yaml");
        if candidate.is_file() {
            return Ok(candidate);
        }
        if !dir.pop() {
            return Err(StateError::WorkspaceNotFound(cwd.to_path_buf()));
        }
    }
}

fn resolve_profile(
    name: &str,
    local: &BTreeMap<String, RawProfile>,
    user: &BTreeMap<String, RawProfile>,
    stack: &mut Vec<String>,
) -> Result<RawProfile, StateError> {
    if stack.iter().any(|seen| seen == name) {
        return Err(StateError::ProfileCycle);
    }
    stack.push(name.to_string());
    let raw = local
        .get(name)
        .or_else(|| user.get(name))
        .cloned()
        .ok_or_else(|| StateError::ProfileNotFound(name.to_string()))?;

    let merged = if let Some(parent) = raw.extends.as_deref() {
        let mut base = resolve_profile(parent, local, user, stack)?;
        if raw.project_id.is_some() {
            base.project_id = raw.project_id.clone();
        }
        if raw.location.is_some() {
            base.location = raw.location.clone();
        }
        base.extends = raw.extends.clone();
        base
    } else {
        raw
    };
    stack.pop();
    Ok(merged)
}

fn location_from(raw: Option<&str>) -> Result<Location, StateError> {
    let Some(raw) = raw else {
        return Err(StateError::LocationRequired);
    };
    Location::new(raw).map_err(|err| match err {
        CoreError::LocationRequired | CoreError::LocationHardcodedGlobalForbidden => {
            StateError::LocationRequired
        }
        other => StateError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            other.to_string(),
        )),
    })
}

fn load_user_profiles() -> BTreeMap<String, RawProfile> {
    let Some(path) = user_profiles_path() else {
        return BTreeMap::new();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return BTreeMap::new();
    };
    parse_user_profiles(&text).unwrap_or_default()
}

fn parse_user_profiles(text: &str) -> Option<BTreeMap<String, RawProfile>> {
    if let Ok(file) = serde_yaml::from_str::<UserProfilesFile>(text) {
        if let Some(profiles) = file.profiles {
            return Some(profiles);
        }
    }
    serde_yaml::from_str::<BTreeMap<String, RawProfile>>(text).ok()
}

fn user_profiles_path() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        let trimmed = xdg.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed).join("cxas").join("profiles.yaml"));
        }
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(
        PathBuf::from(home)
            .join(".config")
            .join("cxas")
            .join("profiles.yaml"),
    )
}
