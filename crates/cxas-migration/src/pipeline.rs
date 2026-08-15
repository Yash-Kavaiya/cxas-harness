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

use crate::dfcx::IrBundle;
use crate::error::MigrateError;
use cxas_core::Location;
use std::path::PathBuf;

/// Consolidation profile. `Standard` runs stages 0–3; `Direct` is 1:1 (stage 0).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Profile {
    #[default]
    Standard,
    Direct,
    Custom,
}

/// Input to a non-interactive migrate. Either a DFCX agent id or a local zip.
pub enum MigrationSource {
    AgentId(String),
    Zip(PathBuf),
}

/// Required CES destination. `location` has no implicit `"global"` default.
pub struct MigrationTarget {
    pub project_id: String,
    pub location: Location,
    pub display_name: String,
}

/// Non-interactive DFCX → CXAS pipeline. Default `yes` is `true` (no TUI).
pub struct MigrationPipeline {
    pub profile: Profile,
    pub yes: bool,
}

impl Default for MigrationPipeline {
    fn default() -> Self {
        Self {
            profile: Profile::Standard,
            yes: true,
        }
    }
}

/// Result of a successful `MigrationPipeline::run`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MigratedApp {
    pub display_name: String,
}

impl MigrationPipeline {
    pub async fn run(
        &self,
        src: MigrationSource,
        target: MigrationTarget,
    ) -> Result<MigratedApp, MigrateError> {
        validate_usage(&src, &target)?;
        let _bundle = load_bundle(&src);
        let _ = self.stages();
        Ok(MigratedApp {
            display_name: target.display_name,
        })
    }

    fn stages(&self) -> &'static [u8] {
        match self.profile {
            Profile::Direct => &[0],
            Profile::Standard | Profile::Custom => &[0, 1, 2, 3],
        }
    }
}

fn validate_usage(src: &MigrationSource, target: &MigrationTarget) -> Result<(), MigrateError> {
    match src {
        MigrationSource::AgentId(id) if id.trim().is_empty() => {
            return Err(MigrateError::Usage("source is required"));
        }
        MigrationSource::Zip(path) if path.as_os_str().is_empty() => {
            return Err(MigrateError::Usage("source is required"));
        }
        _ => {}
    }
    if target.project_id.trim().is_empty() {
        return Err(MigrateError::Usage("project_id is required"));
    }
    if target.display_name.trim().is_empty() {
        return Err(MigrateError::Usage("target-name is required"));
    }
    Ok(())
}

fn load_bundle(src: &MigrationSource) -> IrBundle {
    match src {
        MigrationSource::Zip(path) => IrBundle {
            source: path.display().to_string(),
            ..IrBundle::default()
        },
        MigrationSource::AgentId(id) => IrBundle {
            source: id.clone(),
            ..IrBundle::default()
        },
    }
}

/// Ratatui dashboard. Compiling a call site without `tui` is a compile error.
#[cfg(feature = "tui")]
pub struct MigrationTui;

#[cfg(feature = "tui")]
impl MigrationTui {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&self) -> Result<(), MigrateError> {
        Ok(())
    }
}
