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

//! Splitting a streamed CES response into whole messages as they arrive.
//!
//! `streamRunSession` answers with a JSON array delivered in chunks, and the
//! chunk boundaries have nothing to do with the message boundaries: one read
//! can carry two replies, half a reply, or the middle of a UTF-8 sequence. A
//! caller that buffers to completion before parsing gets correct results and
//! loses the only reason to stream in the first place, so the split has to
//! happen incrementally.
//!
//! This decoder is pure and byte-oriented. Byte-oriented because splitting a
//! chunk at an arbitrary index can land inside a multi-byte character, and
//! decoding each chunk as UTF-8 on arrival would corrupt it; a complete JSON
//! value always ends on an ASCII delimiter, so the conversion is safe once a
//! whole value is in hand.
//!
//! Pure because a stream decoder that can only be exercised against a live
//! service is a stream decoder whose edge cases never get tested. Every case
//! below -- a value split across three chunks, a brace inside a string, a
//! truncated stream -- is a plain function call.

use crate::CoreError;

/// What the decoder is currently in the middle of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// An object or array; ends when nesting returns to zero.
    Nested,
    /// A quoted string; ends at its closing quote.
    Str,
    /// A number, `true`, `false`, or `null`; ends at the next delimiter.
    Bare,
}

/// Incrementally extracts complete JSON values from a streamed array.
///
/// Also accepts newline-delimited values, which is what a caller writing test
/// fixtures by hand will reach for, and what some proxies rewrite a streamed
/// array into.
#[derive(Debug, Default)]
pub struct JsonStreamDecoder {
    buf: Vec<u8>,
    /// Index into `buf` where the value in progress began.
    start: Option<usize>,
    shape: Option<Shape>,
    depth: usize,
    in_string: bool,
    escaped: bool,
    /// Whether the leading `[` that wraps the whole stream has been consumed.
    saw_first_token: bool,
    /// Whether the closing `]` has been seen.
    closed: bool,
}

impl JsonStreamDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed the next chunk and take whatever values it completed.
    ///
    /// Returns only whole values. A chunk that completes nothing returns an
    /// empty vector rather than a partial message, which is the property that
    /// makes it safe to hand each result straight to a JSON parser.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        let mut out = Vec::new();
        for &byte in chunk {
            self.buf.push(byte);
            let index = self.buf.len() - 1;

            if self.in_string {
                if self.escaped {
                    self.escaped = false;
                } else if byte == b'\\' {
                    self.escaped = true;
                } else if byte == b'"' {
                    self.in_string = false;
                    if self.shape == Some(Shape::Str) && self.depth == 0 {
                        out.extend(self.take(index + 1));
                    }
                }
                continue;
            }

            match self.shape {
                None => {
                    if byte.is_ascii_whitespace() || byte == b',' {
                        continue;
                    }
                    if !self.saw_first_token {
                        self.saw_first_token = true;
                        // The array wrapper itself is not a message.
                        if byte == b'[' {
                            continue;
                        }
                    }
                    if byte == b']' {
                        self.closed = true;
                        continue;
                    }
                    self.start = Some(index);
                    match byte {
                        b'{' | b'[' => {
                            self.shape = Some(Shape::Nested);
                            self.depth = 1;
                        }
                        b'"' => {
                            self.shape = Some(Shape::Str);
                            self.in_string = true;
                        }
                        _ => self.shape = Some(Shape::Bare),
                    }
                }
                Some(Shape::Nested) => match byte {
                    b'"' => self.in_string = true,
                    b'{' | b'[' => self.depth += 1,
                    b'}' | b']' => {
                        self.depth -= 1;
                        if self.depth == 0 {
                            out.extend(self.take(index + 1));
                        }
                    }
                    _ => {}
                },
                // Unreachable in practice: the opening quote sets `in_string`,
                // and the branch above owns every byte until the string closes,
                // at which point the value is emitted and `shape` returns to
                // `None`. Written out rather than `unreachable!()` so a future
                // change cannot turn a decoder bug into a panic mid-stream.
                Some(Shape::Str) => {}
                Some(Shape::Bare) => {
                    if byte.is_ascii_whitespace() || byte == b',' || byte == b']' {
                        // The delimiter is not part of the value.
                        let value = self.take(index);
                        self.buf.clear();
                        if byte == b']' {
                            self.closed = true;
                        }
                        out.extend(value);
                    }
                }
            }
        }
        out
    }

    /// Cut the value that ends at `end` out of the buffer.
    fn take(&mut self, end: usize) -> Option<String> {
        let start = self.start.take()?;
        self.shape = None;
        self.depth = 0;
        let bytes = self.buf[start..end].to_vec();
        // A complete JSON value ends on an ASCII delimiter, so no multi-byte
        // sequence can straddle this boundary.
        let value = String::from_utf8(bytes).ok()?;
        self.buf.clear();
        Some(value)
    }

    /// Whether the stream's closing bracket has been seen.
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// Close the stream, returning any value that end-of-input completed.
    ///
    /// A bare literal -- a number, `true`, `null` -- has no closing delimiter,
    /// so end-of-stream is what terminates it. Anything else still in progress
    /// means the connection dropped mid-message, which otherwise looks exactly
    /// like a clean end of stream: the caller would report a truncated
    /// conversation as a complete one.
    pub fn finish(&mut self) -> Result<Option<String>, CoreError> {
        if self.shape == Some(Shape::Bare) {
            let end = self.buf.len();
            return Ok(self.take(end));
        }
        if self.shape.is_some() || self.in_string {
            return Err(CoreError::Transport(format!(
                "stream ended mid-message after {} buffered bytes",
                self.buf.len()
            )));
        }
        Ok(None)
    }
}
