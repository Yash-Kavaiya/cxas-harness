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

//! Typed CES resource names.
//!
//! Every CES path template embeds `projects/*/locations/*`, so a resource name
//! cannot be built without a [`Location`]. That makes issue #401 -- a
//! `vertex_location` hardcoded to `"global"` -- unrepeatable here: there is no
//! constructor that omits the location and no `Default` to fall back to.

use crate::{CoreError, Location};

/// A CES app, and the parent scope its children hang from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppRef {
    project: String,
    location: Location,
    app: String,
}

impl AppRef {
    pub fn new(
        project: impl Into<String>,
        location: Location,
        app: impl Into<String>,
    ) -> Result<Self, CoreError> {
        let project = project.into();
        let app = app.into();
        if project.trim().is_empty() {
            return Err(CoreError::Transport("project id must not be empty".into()));
        }
        if app.trim().is_empty() {
            return Err(CoreError::Transport("app id must not be empty".into()));
        }
        Ok(Self {
            project,
            location,
            app,
        })
    }

    /// `projects/{project}/locations/{location}` -- the parent for app create
    /// and list.
    pub fn location_parent(&self) -> String {
        format!(
            "projects/{}/locations/{}",
            self.project,
            self.location.as_str()
        )
    }

    /// `projects/{project}/locations/{location}/apps/{app}`.
    pub fn name(&self) -> String {
        format!("{}/apps/{}", self.location_parent(), self.app)
    }

    /// The name of a child collection member, e.g. `agents`, `tools`.
    pub fn child(&self, collection: &str, id: &str) -> String {
        format!("{}/{}/{}", self.name(), collection, id)
    }

    pub fn location(&self) -> &Location {
        &self.location
    }

    pub fn project(&self) -> &str {
        &self.project
    }

    pub fn app(&self) -> &str {
        &self.app
    }
}

/// The parent scope for project-and-location-level calls, with no app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationRef {
    project: String,
    location: Location,
}

impl LocationRef {
    pub fn new(project: impl Into<String>, location: Location) -> Result<Self, CoreError> {
        let project = project.into();
        if project.trim().is_empty() {
            return Err(CoreError::Transport("project id must not be empty".into()));
        }
        Ok(Self { project, location })
    }

    pub fn name(&self) -> String {
        format!(
            "projects/{}/locations/{}",
            self.project,
            self.location.as_str()
        )
    }

    pub fn location(&self) -> &Location {
        &self.location
    }
}
