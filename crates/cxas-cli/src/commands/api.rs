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

//! `cxas api` -- the surface that reaches CES itself.
//!
//! The rest of this binary works on a checked-out app directory and its
//! `.cxas` state. This command is the other half: every method CES declares,
//! addressable by its discovery id, with the credential resolved the same way
//! Google's own tools resolve it.
//!
//! Generic rather than one subcommand per method. There are 170 methods and
//! this workspace models 37 of them; a hand-written verb for each of the rest
//! would be 130-odd wrappers that add a spelling to remember and nothing else.
//! What a caller cannot do without help is discover the ids, know which surface
//! declares them, and see which parameters a path needs -- so `list` and
//! `describe` do that, offline, from the same table `call` dispatches through.

use crate::output::{write_err, write_ok, OutputFormat};
use clap::ArgMatches;
use cxas_core::{resolve_method, ApiVersion, MethodSpec, METHODS, MODELLED};
use std::collections::BTreeMap;
use std::io::Write;

pub fn run(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    match matches.subcommand() {
        Some(("list", sub)) => list(sub, format, out),
        Some(("describe", sub)) => describe(sub, format, out),
        Some(("call", sub)) => call(sub, format, out, false),
        Some(("stream", sub)) => call(sub, format, out, true),
        _ => write_err(
            out,
            format,
            "api",
            "USAGE",
            "expected api list, api describe, api call, or api stream",
            2,
        ),
    }
}

/// Render one method as JSON, including what a caller must supply to use it.
fn describe_spec(spec: &MethodSpec) -> serde_json::Value {
    serde_json::json!({
        "id": spec.id,
        "apiVersion": spec.api_version.as_str(),
        "httpMethod": spec.http_method,
        "path": spec.path,
        "parameters": spec.required_params(),
        "streaming": spec.is_streaming(),
        "modelled": MODELLED.contains(&spec.id),
    })
}

/// Which surface the caller asked for, if any. `None` means "either".
fn wanted_version(matches: &ArgMatches) -> Result<Option<ApiVersion>, String> {
    match matches.get_one::<String>("api-version") {
        None => Ok(None),
        Some(raw) => ApiVersion::parse(raw)
            .map(Some)
            .ok_or_else(|| format!("unknown API version {raw:?}; expected v1 or v1beta")),
    }
}

fn list(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let version = match wanted_version(matches) {
        Ok(v) => v,
        Err(message) => return write_err(out, format, "api list", "USAGE", &message, 2),
    };
    let needle = matches.get_one::<String>("filter").cloned();
    let modelled_only = matches.get_flag("modelled");

    let methods: Vec<serde_json::Value> = METHODS
        .iter()
        .filter(|m| match version {
            Some(v) => m.api_version == v,
            None => true,
        })
        .filter(|m| match &needle {
            Some(n) => m.id.contains(n.as_str()),
            None => true,
        })
        .filter(|m| !modelled_only || MODELLED.contains(&m.id))
        .map(describe_spec)
        .collect();

    let human = methods
        .iter()
        .map(|m| {
            format!(
                "{:<6} {:<7} {}",
                m["apiVersion"].as_str().unwrap_or(""),
                m["httpMethod"].as_str().unwrap_or(""),
                m["id"].as_str().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    write_ok(
        out,
        format,
        "api list",
        serde_json::json!({ "count": methods.len(), "methods": methods }),
        &human,
    )
}

fn describe(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write) -> i32 {
    let Some(id) = matches.get_one::<String>("method") else {
        return write_err(
            out,
            format,
            "api describe",
            "USAGE",
            "a method id is required, e.g. ces.projects.locations.apps.list",
            2,
        );
    };
    let Some(spec) = lookup(matches, id, out, format, "api describe") else {
        return 2;
    };

    let data = describe_spec(spec);
    let human = format!(
        "{} {}\n  surface   {}\n  path      {}\n  parameters {}",
        spec.http_method,
        spec.id,
        spec.api_version.as_str(),
        spec.path,
        spec.required_params().join(", ")
    );
    write_ok(out, format, "api describe", data, &human)
}

/// Resolve a method id, reporting the near misses when it does not exist.
fn lookup(
    matches: &ArgMatches,
    id: &str,
    out: &mut impl Write,
    format: OutputFormat,
    command: &str,
) -> Option<&'static MethodSpec> {
    let version = match wanted_version(matches) {
        Ok(v) => v,
        Err(message) => {
            write_err(out, format, command, "USAGE", &message, 2);
            return None;
        }
    };

    let found = match version {
        Some(v) => cxas_core::method_spec(id, v),
        None => resolve_method(id),
    };
    if let Some(spec) = found {
        return Some(spec);
    }

    // The most common miss by far is naming the right method on the wrong
    // surface -- every evaluation method is v1beta-only -- so check that first.
    // Falling straight through to fuzzy matching buries the real answer under
    // five unrelated methods that merely end in the same word.
    if let Some(elsewhere) = METHODS.iter().find(|m| m.id == id) {
        let detail = format!(
            "CES declares {id:?} on {} only, not on {}",
            elsewhere.api_version.as_str(),
            version.map(|v| v.as_str()).unwrap_or("that surface")
        );
        write_err(out, format, command, "UNKNOWN_METHOD", &detail, 2);
        return None;
    }

    // Otherwise a typo. Rank by shared prefix so the suggestions come from the
    // same resource rather than from every resource with a `list`.
    let mut near: Vec<(usize, String)> = METHODS
        .iter()
        .map(|m| {
            let shared = m
                .id
                .chars()
                .zip(id.chars())
                .take_while(|(a, b)| a == b)
                .count();
            (shared, format!("{} ({})", m.id, m.api_version.as_str()))
        })
        .filter(|(shared, _)| *shared > "ces.projects.locations.".len())
        .collect();
    near.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    near.dedup_by(|a, b| a.1 == b.1);
    near.truncate(5);

    let detail = if near.is_empty() {
        format!("no CES method {id:?}; try `cxas api list --filter <text>`")
    } else {
        format!(
            "no CES method {id:?}; did you mean {}",
            near.into_iter().map(|(_, name)| name).collect::<Vec<_>>().join(", ")
        )
    };
    write_err(out, format, command, "UNKNOWN_METHOD", &detail, 2);
    None
}

/// Parse repeated `name=value` flags.
///
/// Only `call` and `stream` use it, so a build without `remote` would report
/// it as dead. Kept outside the feature gate anyway: the parsing rules are the
/// same either way, and moving them would scatter one command across two
/// cfg blocks.
#[cfg_attr(not(feature = "remote"), allow(dead_code))]
fn key_values(matches: &ArgMatches, flag: &str) -> Result<BTreeMap<String, String>, String> {
    let mut map = BTreeMap::new();
    let Some(values) = matches.get_many::<String>(flag) else {
        return Ok(map);
    };
    for raw in values {
        let Some((key, value)) = raw.split_once('=') else {
            return Err(format!("--{flag} expects name=value, got {raw:?}"));
        };
        if key.trim().is_empty() {
            return Err(format!("--{flag} has an empty name in {raw:?}"));
        }
        map.insert(key.trim().to_string(), value.to_string());
    }
    Ok(map)
}

/// Read the request body, from a literal or from `@path`.
#[cfg_attr(not(feature = "remote"), allow(dead_code))]
fn read_body(matches: &ArgMatches) -> Result<Option<String>, String> {
    let Some(raw) = matches.get_one::<String>("body") else {
        return Ok(None);
    };
    let text = match raw.strip_prefix('@') {
        Some(path) => std::fs::read_to_string(path)
            .map_err(|e| format!("reading --body from {path}: {e}"))?,
        None => raw.clone(),
    };
    // Rejected here rather than by CES, so a stray shell quote is a local
    // error with the offending text in hand.
    serde_json::from_str::<serde_json::Value>(&text)
        .map_err(|e| format!("--body is not valid JSON: {e}"))?;
    Ok(Some(text))
}

#[cfg(not(feature = "remote"))]
fn call(_matches: &ArgMatches, format: OutputFormat, out: &mut impl Write, stream: bool) -> i32 {
    let command = if stream { "api stream" } else { "api call" };
    write_err(
        out,
        format,
        command,
        "FEATURE_DISABLED",
        &format!("{command} requires the `remote` feature, which is on by default"),
        2,
    )
}

#[cfg(feature = "remote")]
fn call(matches: &ArgMatches, format: OutputFormat, out: &mut impl Write, stream: bool) -> i32 {
    use crate::transport::block_on;
    use cxas_core::{CesHttpClient, RequestBuilder, TokenProvider};

    let command = if stream { "api stream" } else { "api call" };

    let Some(id) = matches.get_one::<String>("method").cloned() else {
        return write_err(out, format, command, "USAGE", "a method id is required", 2);
    };
    let Some(spec) = lookup(matches, &id, out, format, command) else {
        return 2;
    };

    let params = match key_values(matches, "param") {
        Ok(p) => p,
        Err(message) => return write_err(out, format, command, "USAGE", &message, 2),
    };
    let query = match key_values(matches, "query") {
        Ok(q) => q,
        Err(message) => return write_err(out, format, command, "USAGE", &message, 2),
    };
    let body = match read_body(matches) {
        Ok(b) => b,
        Err(message) => return write_err(out, format, command, "USAGE", &message, 2),
    };

    // Named before the request goes out, so a missing parameter is a usage
    // error naming the parameter rather than a 404 from CES.
    let missing: Vec<&str> = spec
        .required_params()
        .into_iter()
        .filter(|name| !params.contains_key(*name))
        .collect();
    if !missing.is_empty() {
        return write_err(
            out,
            format,
            command,
            "MISSING_PARAMETER",
            &format!(
                "{} needs {} -- supply each with --param name=value",
                spec.id,
                missing.join(", ")
            ),
            2,
        );
    }

    if stream && !spec.is_streaming() {
        return write_err(
            out,
            format,
            command,
            "NOT_STREAMING",
            &format!("{} returns a single response; use `cxas api call`", spec.id),
            2,
        );
    }

    let endpoint = matches
        .get_one::<String>("endpoint")
        .cloned()
        .unwrap_or_else(|| cxas_core::DEFAULT_ENDPOINT.to_string());
    let explicit = matches.get_one::<String>("oauth-token").cloned();

    let provider = match TokenProvider::discover(explicit.as_deref()) {
        Ok(p) => p,
        Err(e) => return write_err(out, format, command, "AUTH", &e.to_string(), 1),
    };
    let credential = provider.source().label().to_string();

    let client = match CesHttpClient::with_builder(RequestBuilder::new(endpoint)) {
        Ok(c) => c.with_tokens(provider),
        Err(e) => return write_err(out, format, command, "TRANSPORT", &e.to_string(), 1),
    };

    if stream {
        let spec = *spec;
        let outcome = block_on(async move {
            let mut messages = Vec::new();
            let result = client
                .stream(&spec, &params, &query, body, |message| {
                    messages.push(message.to_string())
                })
                .await;
            (result, messages)
        });

        let (result, messages) = outcome;
        // Whatever arrived is reported even when the stream then failed: a
        // partial transcript is evidence, and discarding it on error would
        // throw away the only record of what the agent said.
        let parsed: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| serde_json::from_str(m).unwrap_or(serde_json::Value::String(m.clone())))
            .collect();

        return match result {
            Ok(count) => write_ok(
                out,
                format,
                command,
                serde_json::json!({
                    "method": spec.id,
                    "credential": credential,
                    "messages": parsed,
                    "count": count,
                }),
                &messages.join("\n"),
            ),
            Err(e) => write_err_with_partial(out, format, command, &e.to_string(), parsed),
        };
    }

    let spec = *spec;
    let result = block_on(async move { client.call(&spec, &params, &query, body).await });

    match result {
        Ok(text) => {
            let data = serde_json::from_str(&text).unwrap_or(serde_json::Value::String(text.clone()));
            write_ok(
                out,
                format,
                command,
                serde_json::json!({
                    "method": spec.id,
                    "credential": credential,
                    "response": data,
                }),
                &text,
            )
        }
        Err(e) => write_err(out, format, command, "CES", &e.to_string(), 1),
    }
}

#[cfg(feature = "remote")]
fn write_err_with_partial(
    out: &mut impl Write,
    format: OutputFormat,
    command: &str,
    message: &str,
    messages: Vec<serde_json::Value>,
) -> i32 {
    crate::output::write_err_with_data(
        out,
        format,
        command,
        "CES",
        message,
        Some(serde_json::json!({ "messages": messages, "partial": true })),
        1,
    )
}
