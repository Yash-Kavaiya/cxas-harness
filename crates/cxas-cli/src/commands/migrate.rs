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

use crate::commands::{opt_str, resolve_location};
use crate::output::{write_err, write_ok, OutputFormat};
use crate::transport::block_on;
use clap::ArgMatches;
use cxas_core::Location;
use cxas_migration::{MigrateError, MigrationPipeline, MigrationSource, MigrationTarget, Profile};
use std::io::Write;
use std::path::PathBuf;

pub fn run(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let Some(location) = resolve_location(matches, None) else {
        return write_err(
            out,
            format,
            "migrate dfcx",
            "LOCATION_REQUIRED",
            "location is required and has no default",
            2,
        );
    };
    let loc = match Location::new(&location) {
        Ok(loc) => loc,
        Err(_) => {
            return write_err(
                out,
                format,
                "migrate dfcx",
                "LOCATION_REQUIRED",
                "location is required and has no default",
                2,
            );
        }
    };
    let src = if let Some(zip) = opt_str(matches, "zip") {
        MigrationSource::Zip(PathBuf::from(zip))
    } else if let Some(id) = opt_str(matches, "source").or_else(|| opt_str(matches, "agent-id")) {
        MigrationSource::AgentId(id)
    } else {
        return write_err(
            out,
            format,
            "migrate dfcx",
            "USAGE",
            "source is required",
            2,
        );
    };
    let project_id = opt_str(matches, "project-id").unwrap_or_default();
    let display_name = opt_str(matches, "target-name")
        .or_else(|| opt_str(matches, "display-name"))
        .unwrap_or_default();
    let profile = match opt_str(matches, "profile")
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "direct" => Profile::Direct,
        "custom" => Profile::Custom,
        _ => Profile::Standard,
    };
    let pipeline = MigrationPipeline { profile, yes: true };
    let result = block_on(async move {
        pipeline
            .run(
                src,
                MigrationTarget {
                    project_id,
                    location: loc,
                    display_name,
                },
            )
            .await
    });
    match result {
        Ok(app) => write_ok(
            out,
            format,
            "migrate dfcx",
            serde_json::json!({ "display_name": app.display_name }),
            "migrated",
        ),
        Err(MigrateError::Usage(msg)) => write_err(out, format, "migrate dfcx", "USAGE", msg, 2),
        Err(MigrateError::FeatureDisabled(feat)) => write_err(
            out,
            format,
            "migrate dfcx",
            "FEATURE_DISABLED",
            &format!("feature {feat} is not enabled"),
            2,
        ),
        Err(err) => write_err(out, format, "migrate dfcx", "MIGRATE", &err.to_string(), 1),
    }
}
