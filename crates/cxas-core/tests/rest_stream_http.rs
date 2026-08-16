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

//! `streamRunSession` over a real socket.
//!
//! The decoder is covered exhaustively in `rest_stream.rs`; what is left to
//! prove is that the client wires it to the response body correctly -- that
//! messages surface as they arrive rather than at the end, and that a
//! connection dying mid-message is reported instead of rounded down to a
//! shorter conversation.

#![cfg(feature = "rest")]

mod support;

use cxas_core::{
    method_spec, ApiVersion, AppRef, CesHttpClient, Location, RequestBuilder,
};
use std::collections::BTreeMap;
use std::io::Write;
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// How long the server waits for the client to acknowledge a message before
/// giving up. Generous: it only elapses when delivery is *not* incremental,
/// and a hung test is worse than a failed one.
const ACK_TIMEOUT: Duration = Duration::from_secs(5);

/// Serve one streaming response, writing `pieces` in order.
///
/// The body carries no `content-length` and is not chunk-framed; it ends when
/// the connection closes, which is what lets `truncate` produce a stream that
/// stops mid-message.
fn stream_server(
    pieces: Vec<String>,
    truncate: bool,
    ack: Option<Receiver<()>>,
) -> (String, Arc<Mutex<bool>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let acknowledged = Arc::new(Mutex::new(true));
    let flag = Arc::clone(&acknowledged);

    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let _ = support::read_request(&mut stream);

        let header = "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\nconnection: close\r\n\r\n";
        stream.write_all(header.as_bytes()).expect("header");
        stream.flush().expect("flush header");

        for (index, piece) in pieces.iter().enumerate() {
            stream.write_all(piece.as_bytes()).expect("piece");
            stream.flush().expect("flush piece");

            // Before writing the next piece, wait for the client to report the
            // previous one. If it cannot, delivery is not incremental.
            if index + 1 < pieces.len() {
                if let Some(rx) = &ack {
                    if rx.recv_timeout(ACK_TIMEOUT).is_err() {
                        *flag.lock().expect("flag") = false;
                    }
                }
            }
        }

        if !truncate {
            stream.write_all(b"]").expect("terminator");
        }
        stream.flush().ok();
        // Dropping the stream closes the connection, ending the body.
    });

    (format!("http://{addr}"), acknowledged)
}

fn session_spec() -> &'static cxas_core::MethodSpec {
    method_spec(
        "ces.projects.locations.apps.sessions.streamRunSession",
        ApiVersion::V1,
    )
    .expect("streamRunSession is declared on v1")
}

fn session_params() -> BTreeMap<String, String> {
    let app = AppRef::new("proj", Location::new("us-central1").unwrap(), "demo").expect("app");
    [(
        "session".to_string(),
        format!("{}/sessions/session-1", app.name()),
    )]
    .into_iter()
    .collect()
}

#[tokio::test]
async fn each_message_is_delivered_as_it_arrives_not_at_the_end() {
    // The property that distinguishes streaming from a slow unary call. The
    // server refuses to send message two until message one has been reported,
    // so a client that buffered the whole body would stall here.
    let (ack_tx, ack_rx): (Sender<()>, Receiver<()>) = mpsc::channel();
    let (endpoint, incremental) = stream_server(
        vec![
            r#"[{"reply":"one"}"#.to_string(),
            r#",{"reply":"two"}"#.to_string(),
            r#",{"reply":"three"}"#.to_string(),
        ],
        false,
        Some(ack_rx),
    );

    let client = CesHttpClient::with_builder(RequestBuilder::new(endpoint)).expect("client");
    let seen = Arc::new(Mutex::new(Vec::new()));
    let collector = Arc::clone(&seen);

    let delivered = client
        .stream(
            session_spec(),
            &session_params(),
            &BTreeMap::new(),
            Some(r#"{"query":"hello"}"#.to_string()),
            move |message| {
                collector.lock().expect("seen").push(message.to_string());
                let _ = ack_tx.send(());
            },
        )
        .await
        .expect("stream must complete");

    assert_eq!(delivered, 3);
    assert_eq!(
        *seen.lock().expect("seen"),
        vec![
            r#"{"reply":"one"}"#,
            r#"{"reply":"two"}"#,
            r#"{"reply":"three"}"#
        ]
    );
    assert!(
        *incremental.lock().expect("flag"),
        "the client did not report a message until the whole body had arrived"
    );
}

#[tokio::test]
async fn a_stream_cut_short_is_an_error_and_keeps_what_arrived() {
    // A dropped connection must not read as a completed conversation.
    let (endpoint, _) = stream_server(
        vec![r#"[{"reply":"one"},{"reply":"tw"#.to_string()],
        true,
        None,
    );

    let client = CesHttpClient::with_builder(RequestBuilder::new(endpoint)).expect("client");
    let seen = Arc::new(Mutex::new(Vec::new()));
    let collector = Arc::clone(&seen);

    let err = client
        .stream(
            session_spec(),
            &session_params(),
            &BTreeMap::new(),
            None,
            move |message| collector.lock().expect("seen").push(message.to_string()),
        )
        .await
        .expect_err("a truncated stream must be reported");

    assert!(err.to_string().contains("mid-message"), "got {err}");
    assert_eq!(
        *seen.lock().expect("seen"),
        vec![r#"{"reply":"one"}"#],
        "the message that did arrive whole should still have been delivered"
    );
}

#[tokio::test]
async fn a_rejected_stream_fails_before_any_message_is_delivered() {
    let stub = support::serve(vec![support::Reply::status(
        403,
        r#"{"error":{"message":"caller lacks ces.sessions.run"}}"#,
    )]);
    let client = CesHttpClient::with_builder(RequestBuilder::new(&stub.url)).expect("client");

    let mut delivered = 0;
    let err = client
        .stream(
            session_spec(),
            &session_params(),
            &BTreeMap::new(),
            None,
            |_| delivered += 1,
        )
        .await
        .expect_err("403 must be an error");

    assert_eq!(delivered, 0);
    assert!(err.to_string().contains("ces.sessions.run"), "got {err}");
}

#[tokio::test]
async fn the_streaming_request_addresses_the_session_resource() {
    let stub = support::serve(vec![support::Reply::ok("[]")]);
    let client = CesHttpClient::with_builder(RequestBuilder::new(&stub.url)).expect("client");

    let delivered = client
        .stream(
            session_spec(),
            &session_params(),
            &BTreeMap::new(),
            Some(r#"{"query":"hi"}"#.to_string()),
            |_| {},
        )
        .await
        .expect("empty stream is not an error");

    assert_eq!(delivered, 0);
    let got = stub.next_request();
    assert_eq!(
        got.request_line,
        "POST /v1/projects/proj/locations/us-central1/apps/demo/sessions/session-1:streamRunSession HTTP/1.1"
    );
    assert_eq!(got.body, r#"{"query":"hi"}"#);
}

#[test]
fn only_the_streaming_method_is_marked_as_streaming() {
    assert!(session_spec().is_streaming());
    assert!(
        !method_spec("ces.projects.locations.apps.sessions.runSession", ApiVersion::V1)
            .expect("runSession")
            .is_streaming(),
        "the unary session call must not be routed through the stream decoder"
    );
}
