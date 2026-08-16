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

//! A scripted HTTP server on loopback, for tests that need a real socket.
//!
//! Real sockets rather than a mocked client because the questions worth asking
//! -- did the bearer header actually go out, did the body arrive intact, does a
//! 403 become an error -- are about what reaches the wire. A mock that
//! intercepts above the transport cannot answer any of them.

#![allow(dead_code)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;

/// What the stub actually received.
#[derive(Debug, Clone)]
pub struct Captured {
    pub request_line: String,
    pub headers: Vec<String>,
    pub body: String,
}

impl Captured {
    /// Case-insensitive header lookup, returning the value only.
    pub fn header(&self, name: &str) -> Option<String> {
        let prefix = format!("{}:", name.to_ascii_lowercase());
        self.headers.iter().find_map(|h| {
            let lower = h.to_ascii_lowercase();
            lower
                .strip_prefix(&prefix)
                .map(|value| h[h.len() - value.len()..].trim().to_string())
        })
    }

    pub fn has_header(&self, name: &str, value: &str) -> bool {
        self.header(name)
            .is_some_and(|actual| actual.eq_ignore_ascii_case(value))
    }
}

/// One scripted reply.
#[derive(Debug, Clone)]
pub struct Reply {
    pub status: u16,
    pub body: String,
}

impl Reply {
    pub fn ok(body: &str) -> Self {
        Self {
            status: 200,
            body: body.to_string(),
        }
    }

    pub fn status(status: u16, body: &str) -> Self {
        Self {
            status,
            body: body.to_string(),
        }
    }
}

/// A running stub. Dropping it leaves the thread to exit on its own.
pub struct Stub {
    pub url: String,
    received: mpsc::Receiver<Captured>,
}

impl Stub {
    /// The next request the stub handled. Panics rather than blocking forever.
    pub fn next_request(&self) -> Captured {
        self.received
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("the stub should have received a request")
    }

    /// True when no further request arrived, proving a cache was used.
    pub fn saw_no_further_request(&self) -> bool {
        self.received
            .recv_timeout(std::time::Duration::from_millis(250))
            .is_err()
    }
}

/// Read one HTTP request off `stream`.
pub fn read_request(stream: &mut std::net::TcpStream) -> Captured {
    let mut reader = BufReader::new(stream.try_clone().expect("clone"));

    let mut request_line = String::new();
    reader.read_line(&mut request_line).expect("request line");

    let mut headers = Vec::new();
    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).expect("header") == 0 {
            break;
        }
        let trimmed = line.trim_end().to_string();
        if trimmed.is_empty() {
            break;
        }
        if let Some(v) = trimmed
            .to_ascii_lowercase()
            .strip_prefix("content-length:")
        {
            content_length = v.trim().parse().unwrap_or(0);
        }
        headers.push(trimmed);
    }

    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body).expect("body");
    }

    Captured {
        request_line: request_line.trim_end().to_string(),
        headers,
        body: String::from_utf8_lossy(&body).to_string(),
    }
}

/// Serve `replies` in order, one per connection, then stop.
pub fn serve(replies: Vec<Reply>) -> Stub {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        for reply in replies {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let captured = read_request(&mut stream);
            let reason = if (200..300).contains(&reply.status) {
                "OK"
            } else {
                "ERR"
            };
            let response = format!(
                "HTTP/1.1 {} {reason}\r\ncontent-length: {}\r\ncontent-type: application/json\r\nconnection: close\r\n\r\n{}",
                reply.status,
                reply.body.len(),
                reply.body
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
            if tx.send(captured).is_err() {
                return;
            }
        }
    });

    Stub {
        url: format!("http://{addr}"),
        received: rx,
    }
}
