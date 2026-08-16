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

//! `cxas api`, end to end.
//!
//! The offline half runs against the generated method table. The live half
//! runs against a stub on loopback with an explicit token, so it exercises the
//! real request path without a credential or a network.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;

fn run(args: &[&str]) -> (i32, serde_json::Value) {
    let mut argv = vec!["cxas".to_string()];
    argv.extend(args.iter().map(|s| s.to_string()));
    let mut buf = std::io::Cursor::new(Vec::new());
    let code = cxas_cli::run(&argv, &mut buf);
    let text = String::from_utf8(buf.into_inner()).expect("utf-8");
    let line = text.lines().last().unwrap_or(&text).to_string();
    let value = serde_json::from_str(&line).unwrap_or_else(|_| {
        serde_json::json!({ "raw": text, "ok": false, "error": { "code": "PARSE" } })
    });
    (code, value)
}

/// Serve one request, reply with `status` and `payload`, report the request line.
fn stub(status: u16, payload: &'static str) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut reader = BufReader::new(stream.try_clone().expect("clone"));
        let mut request_line = String::new();
        reader.read_line(&mut request_line).expect("request line");

        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).expect("header") == 0 {
                break;
            }
            if line.trim_end().is_empty() {
                break;
            }
            if let Some(v) = line
                .to_ascii_lowercase()
                .trim_end()
                .strip_prefix("content-length:")
            {
                content_length = v.trim().parse().unwrap_or(0);
            }
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
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
        let _ = tx.send(request_line.trim_end().to_string());
    });

    (format!("http://{addr}"), rx)
}

const APP: &str = "projects/proj/locations/us-central1/apps/demo";

#[test]
fn list_reports_every_method_ces_declares() {
    let (code, value) = run(&["api", "list"]);
    assert_eq!(code, 0);
    assert_eq!(value["data"]["count"], 170);
}

#[test]
fn list_can_be_narrowed_to_one_surface() {
    let (_, v1) = run(&["api", "list", "--api-version", "v1"]);
    let (_, beta) = run(&["api", "list", "--api-version", "v1beta"]);
    assert_eq!(v1["data"]["count"], 66);
    assert_eq!(beta["data"]["count"], 104);
}

#[test]
fn every_evaluation_method_listed_is_on_the_beta_surface() {
    // The mistake this catches is real: v1 declares no evaluation resources at
    // all, so a v1 evaluation URL can only ever 404.
    let (_, value) = run(&["api", "list", "--filter", "evaluation"]);
    let methods = value["data"]["methods"].as_array().expect("methods");
    assert!(!methods.is_empty());
    for method in methods {
        assert_eq!(
            method["apiVersion"], "v1beta",
            "{} was listed on v1",
            method["id"]
        );
    }
}

#[test]
fn modelled_methods_are_a_strict_subset_of_addressable_ones() {
    let (_, all) = run(&["api", "list"]);
    let (_, modelled) = run(&["api", "list", "--modelled"]);
    let total = all["data"]["count"].as_u64().expect("count");
    let subset = modelled["data"]["count"].as_u64().expect("count");
    assert!(subset > 0 && subset < total, "{subset} of {total}");
}

#[test]
fn describe_names_the_parameters_a_path_needs() {
    let (code, value) = run(&["api", "describe", "ces.projects.locations.apps.list"]);
    assert_eq!(code, 0);
    assert_eq!(value["data"]["httpMethod"], "GET");
    assert_eq!(value["data"]["path"], "v1/{+parent}/apps");
    assert_eq!(value["data"]["parameters"][0], "parent");
}

#[test]
fn describing_an_unknown_method_suggests_the_near_misses() {
    let (code, value) = run(&["api", "describe", "ces.projects.locations.apps.lst"]);
    assert_eq!(code, 2);
    assert_eq!(value["error"]["code"], "UNKNOWN_METHOD");
    // Suggestions ranked by shared prefix, so they come from the same resource
    // rather than from every resource with a method of the same name.
    let message = value["error"]["message"].as_str().expect("message");
    assert!(
        message.contains("ces.projects.locations.apps.list"),
        "got {message}"
    );
}

#[test]
fn asking_for_an_evaluation_method_on_v1_says_where_it_actually_lives() {
    let (code, value) = run(&[
        "api",
        "describe",
        "ces.projects.locations.apps.evaluationRuns.list",
        "--api-version",
        "v1",
    ]);
    assert_eq!(code, 2);
    let message = value["error"]["message"].as_str().expect("message");
    assert!(
        message.contains("v1beta") && message.contains("not on v1"),
        "the error must say where the method actually lives: {message}"
    );
}

#[test]
fn a_missing_path_parameter_is_named_before_anything_is_sent() {
    // Reported without resolving a credential or opening a socket: the caller
    // gets the parameter name, not a 404 from CES.
    let (code, value) = run(&["api", "call", "ces.projects.locations.apps.get"]);
    assert_eq!(code, 2);
    assert_eq!(value["error"]["code"], "MISSING_PARAMETER");
    assert!(
        value["error"]["message"]
            .as_str()
            .expect("message")
            .contains("name"),
        "got {}",
        value["error"]["message"]
    );
}

#[test]
fn a_malformed_param_flag_is_a_usage_error() {
    let (code, value) = run(&[
        "api",
        "call",
        "ces.projects.locations.apps.get",
        "--param",
        "name",
    ]);
    assert_eq!(code, 2);
    assert_eq!(value["error"]["code"], "USAGE");
}

#[test]
fn a_body_that_is_not_json_is_rejected_locally() {
    let (code, value) = run(&[
        "api",
        "call",
        "ces.projects.locations.apps.create",
        "--param",
        "parent=projects/p/locations/us-central1",
        "--body",
        "{displayName: demo}",
    ]);
    assert_eq!(code, 2);
    assert_eq!(value["error"]["code"], "USAGE");
    assert!(value["error"]["message"]
        .as_str()
        .expect("message")
        .contains("valid JSON"));
}

#[test]
fn streaming_a_unary_method_is_refused_rather_than_hanging() {
    let (code, value) = run(&[
        "api",
        "stream",
        "ces.projects.locations.apps.sessions.runSession",
        "--param",
        &format!("session={APP}/sessions/s1"),
    ]);
    assert_eq!(code, 2);
    assert_eq!(value["error"]["code"], "NOT_STREAMING");
}

#[test]
fn a_call_reaches_the_endpoint_and_returns_the_response() {
    let (endpoint, rx) = stub(200, r#"{"name":"apps/demo","displayName":"Demo"}"#);
    let (code, value) = run(&[
        "api",
        "call",
        "ces.projects.locations.apps.get",
        "--param",
        &format!("name={APP}"),
        "--endpoint",
        &endpoint,
        "--oauth-token",
        "tok",
    ]);

    assert_eq!(code, 0, "got {value}");
    assert_eq!(value["data"]["response"]["displayName"], "Demo");
    assert_eq!(value["data"]["credential"], "static-token");
    assert_eq!(
        rx.recv().expect("captured"),
        "GET /v1/projects/proj/locations/us-central1/apps/demo HTTP/1.1"
    );
}

#[test]
fn a_rejected_call_exits_nonzero_with_the_servers_reason() {
    let (endpoint, _rx) = stub(403, r#"{"error":{"message":"location not permitted"}}"#);
    let (code, value) = run(&[
        "api",
        "call",
        "ces.projects.locations.apps.get",
        "--param",
        &format!("name={APP}"),
        "--endpoint",
        &endpoint,
        "--oauth-token",
        "tok",
    ]);

    assert_eq!(code, 1, "a refused call must not exit 0");
    assert_eq!(value["ok"], false);
    assert!(value["error"]["message"]
        .as_str()
        .expect("message")
        .contains("location not permitted"));
}

#[test]
fn a_stream_prints_each_message_it_received() {
    let (endpoint, rx) = stub(200, r#"[{"reply":"one"},{"reply":"two"}]"#);
    let (code, value) = run(&[
        "api",
        "stream",
        "ces.projects.locations.apps.sessions.streamRunSession",
        "--param",
        &format!("session={APP}/sessions/s1"),
        "--body",
        r#"{"query":"hi"}"#,
        "--endpoint",
        &endpoint,
        "--oauth-token",
        "tok",
    ]);

    assert_eq!(code, 0, "got {value}");
    assert_eq!(value["data"]["count"], 2);
    assert_eq!(value["data"]["messages"][1]["reply"], "two");
    assert!(rx
        .recv()
        .expect("captured")
        .contains(":streamRunSession"));
}

#[test]
fn a_query_parameter_reaches_the_url() {
    let (endpoint, rx) = stub(200, r#"{"apps":[]}"#);
    let (code, _) = run(&[
        "api",
        "call",
        "ces.projects.locations.apps.list",
        "--param",
        "parent=projects/proj/locations/us-central1",
        "--query",
        "pageSize=5",
        "--endpoint",
        &endpoint,
        "--oauth-token",
        "tok",
    ]);
    assert_eq!(code, 0);
    assert!(rx.recv().expect("captured").contains("?pageSize=5"));
}
