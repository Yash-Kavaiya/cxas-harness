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

//! Exercises the real HTTP path against a stub server on loopback.
//!
//! No network and no credentials: the point is to prove the request this crate
//! puts on the wire is the one the discovery document describes, and that a
//! non-2xx status becomes a typed error rather than a success carrying an
//! error payload.

#![cfg(feature = "rest")]

use cxas_core::{
    method_spec, ApiVersion, AppRef, CesHttpClient, Location, RequestBuilder,
};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;

/// The request line and headers the stub actually received.
struct Captured {
    request_line: String,
    headers: Vec<String>,
    body: String,
}

/// Serve exactly one request, reply with `status` and `payload`, and report
/// back what was received.
fn stub_server(status: u16, payload: &'static str) -> (String, mpsc::Receiver<Captured>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut reader = BufReader::new(stream.try_clone().expect("clone"));

        let mut request_line = String::new();
        reader.read_line(&mut request_line).expect("request line");

        let mut headers = Vec::new();
        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("header");
            let trimmed = line.trim_end().to_string();
            if trimmed.is_empty() {
                break;
            }
            if let Some(v) = trimmed.to_ascii_lowercase().strip_prefix("content-length:") {
                content_length = v.trim().parse().unwrap_or(0);
            }
            headers.push(trimmed);
        }

        let mut body = vec![0u8; content_length];
        if content_length > 0 {
            reader.read_exact(&mut body).expect("body");
        }

        let reason = if (200..300).contains(&status) { "OK" } else { "ERR" };
        let response = format!(
            "HTTP/1.1 {status} {reason}\r\ncontent-length: {}\r\ncontent-type: application/json\r\nconnection: close\r\n\r\n{payload}",
            payload.len()
        );
        stream.write_all(response.as_bytes()).expect("write");
        stream.flush().ok();

        tx.send(Captured {
            request_line: request_line.trim_end().to_string(),
            headers,
            body: String::from_utf8_lossy(&body).to_string(),
        })
        .ok();
    });

    (format!("http://{addr}"), rx)
}

fn app() -> AppRef {
    AppRef::new("proj", Location::new("us-central1").unwrap(), "demo").expect("app")
}

fn params(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[tokio::test]
async fn get_app_puts_the_documented_request_on_the_wire() {
    let (endpoint, rx) = stub_server(200, r#"{"name":"projects/proj/locations/us-central1/apps/demo"}"#);
    let client =
        CesHttpClient::with_builder(RequestBuilder::new(endpoint).with_token("tok")).expect("client");
    let spec = method_spec("ces.projects.locations.apps.get", ApiVersion::V1).expect("spec");

    let body = client
        .call(spec, &params(&[("name", &app().name())]), &BTreeMap::new(), None)
        .await
        .expect("call must succeed");

    assert!(body.contains("apps/demo"));

    let got = rx.recv().expect("stub captured a request");
    assert_eq!(
        got.request_line,
        "GET /v1/projects/proj/locations/us-central1/apps/demo HTTP/1.1"
    );
    assert!(
        got.headers
            .iter()
            .any(|h| h.eq_ignore_ascii_case("authorization: Bearer tok")),
        "missing bearer token, got {:?}",
        got.headers
    );
}

#[tokio::test]
async fn create_app_sends_the_json_body() {
    let (endpoint, rx) = stub_server(200, r#"{"name":"x"}"#);
    let client = CesHttpClient::with_builder(RequestBuilder::new(endpoint)).expect("client");
    let spec = method_spec("ces.projects.locations.apps.create", ApiVersion::V1).expect("spec");

    client
        .call(
            spec,
            &params(&[("parent", &app().location_parent())]),
            &BTreeMap::new(),
            Some(r#"{"displayName":"demo"}"#.to_string()),
        )
        .await
        .expect("call must succeed");

    let got = rx.recv().expect("captured");
    assert_eq!(
        got.request_line,
        "POST /v1/projects/proj/locations/us-central1/apps HTTP/1.1"
    );
    assert_eq!(got.body, r#"{"displayName":"demo"}"#);
}

#[tokio::test]
async fn evaluation_run_is_addressed_on_the_v1beta_surface() {
    // Evaluations exist only on v1beta; the URL must say so.
    let (endpoint, rx) = stub_server(200, r#"{"evaluationRuns":[]}"#);
    let client = CesHttpClient::with_builder(RequestBuilder::new(endpoint)).expect("client");
    let spec = method_spec(
        "ces.projects.locations.apps.evaluationRuns.list",
        ApiVersion::V1Beta,
    )
    .expect("spec");

    client
        .call(spec, &params(&[("parent", &app().name())]), &BTreeMap::new(), None)
        .await
        .expect("call must succeed");

    let got = rx.recv().expect("captured");
    assert_eq!(
        got.request_line,
        "GET /v1beta/projects/proj/locations/us-central1/apps/demo/evaluationRuns HTTP/1.1"
    );
}

#[tokio::test]
async fn a_403_becomes_a_typed_error_not_a_successful_body() {
    // A residency policy rejection must not be returned as if it were data.
    let (endpoint, _rx) = stub_server(403, r#"{"error":{"message":"location not permitted"}}"#);
    let client = CesHttpClient::with_builder(RequestBuilder::new(endpoint)).expect("client");
    let spec = method_spec("ces.projects.locations.apps.get", ApiVersion::V1).expect("spec");

    let err = client
        .call(spec, &params(&[("name", &app().name())]), &BTreeMap::new(), None)
        .await
        .expect_err("403 must be an error");

    let msg = format!("{err}");
    assert!(msg.contains("403"), "got {msg}");
    assert!(msg.contains("location not permitted"), "got {msg}");
}

#[tokio::test]
async fn a_429_reports_quota_exhaustion() {
    // #263: evaluation runs contending with the general session quota should
    // surface as quota, not as a generic failure.
    let (endpoint, _rx) = stub_server(429, r#"{"error":{"message":"RunSession quota"}}"#);
    let client = CesHttpClient::with_builder(RequestBuilder::new(endpoint)).expect("client");
    let spec = method_spec("ces.projects.locations.apps.get", ApiVersion::V1).expect("spec");

    let err = client
        .call(spec, &params(&[("name", &app().name())]), &BTreeMap::new(), None)
        .await
        .expect_err("429 must be an error");
    assert!(format!("{err}").contains("quota"), "got {err}");
}

#[tokio::test]
async fn query_parameters_reach_the_wire() {
    let (endpoint, rx) = stub_server(200, "{}");
    let client = CesHttpClient::with_builder(RequestBuilder::new(endpoint)).expect("client");
    let spec = method_spec("ces.projects.locations.apps.list", ApiVersion::V1).expect("spec");

    client
        .call(
            spec,
            &params(&[("parent", &app().location_parent())]),
            &params(&[("pageSize", "25")]),
            None,
        )
        .await
        .expect("call must succeed");

    let got = rx.recv().expect("captured");
    assert!(got.request_line.contains("?pageSize=25"), "got {}", got.request_line);
}
