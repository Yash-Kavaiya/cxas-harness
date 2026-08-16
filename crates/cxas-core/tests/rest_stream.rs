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

//! The streamed-response decoder, exercised at the boundaries that break it.
//!
//! The interesting cases are all about chunk boundaries falling somewhere
//! inconvenient, which is exactly what a live service does at random and what
//! a live-service test therefore cannot reproduce on demand.

use cxas_core::JsonStreamDecoder;

/// Feed `input` one byte at a time -- the most hostile chunking there is.
fn byte_at_a_time(input: &str) -> Vec<String> {
    let mut decoder = JsonStreamDecoder::new();
    let mut out = Vec::new();
    for byte in input.as_bytes() {
        out.extend(decoder.push(&[*byte]));
    }
    out.extend(decoder.finish().expect("clean end"));
    out
}

#[test]
fn a_whole_array_in_one_chunk_yields_each_element() {
    let mut decoder = JsonStreamDecoder::new();
    let got = decoder.push(br#"[{"a":1},{"b":2}]"#);
    assert_eq!(got, vec![r#"{"a":1}"#, r#"{"b":2}"#]);
    assert!(decoder.is_closed());
}

#[test]
fn a_message_split_across_chunks_is_never_delivered_in_pieces() {
    // The property that makes the result safe to hand to a JSON parser.
    let mut decoder = JsonStreamDecoder::new();
    assert!(decoder.push(br#"[{"text":"hel"#).is_empty());
    assert!(decoder.push(br#"lo wor"#).is_empty());
    assert_eq!(decoder.push(br#"ld"}]"#), vec![r#"{"text":"hello world"}"#]);
}

#[test]
fn a_brace_inside_a_string_does_not_end_the_message() {
    // A transcript containing JSON, which a conversational API returns often.
    let mut decoder = JsonStreamDecoder::new();
    let got = decoder.push(br#"[{"text":"use {\"a\": 1} here"}]"#);
    assert_eq!(got, vec![r#"{"text":"use {\"a\": 1} here"}"#]);
}

#[test]
fn an_escaped_backslash_before_a_quote_still_closes_the_string() {
    // `"...\\"` ends the string; `"...\"` does not. Getting this backwards
    // swallows the rest of the stream into one enormous "message".
    let mut decoder = JsonStreamDecoder::new();
    let got = decoder.push(br#"[{"path":"C:\\"},{"next":true}]"#);
    assert_eq!(got, vec![r#"{"path":"C:\\"}"#, r#"{"next":true}"#]);
}

#[test]
fn nested_objects_and_arrays_do_not_end_the_message_early() {
    let mut decoder = JsonStreamDecoder::new();
    let payload = r#"[{"turn":{"parts":[{"t":"a"},{"t":"b"}]}},{"done":true}]"#;
    let got = decoder.push(payload.as_bytes());
    assert_eq!(
        got,
        vec![r#"{"turn":{"parts":[{"t":"a"},{"t":"b"}]}}"#, r#"{"done":true}"#]
    );
}

#[test]
fn a_multibyte_character_split_across_chunks_survives() {
    // A chunk boundary can land inside a UTF-8 sequence. Decoding each chunk
    // as text on arrival would replace it with U+FFFD and corrupt the reply.
    let message = r#"[{"text":"héllo — ok"}]"#;
    let bytes = message.as_bytes();
    let mut decoder = JsonStreamDecoder::new();
    let mut got = Vec::new();
    for chunk in bytes.chunks(3) {
        got.extend(decoder.push(chunk));
    }
    assert_eq!(got, vec![r#"{"text":"héllo — ok"}"#]);
}

#[test]
fn byte_at_a_time_delivery_matches_whole_delivery() {
    assert_eq!(
        byte_at_a_time(r#"[{"a":1},{"b":[2,3]},{"c":"x"}]"#),
        vec![r#"{"a":1}"#, r#"{"b":[2,3]}"#, r#"{"c":"x"}"#]
    );
}

#[test]
fn whitespace_and_newlines_between_messages_are_ignored() {
    let mut decoder = JsonStreamDecoder::new();
    let got = decoder.push(b"[\n  {\"a\":1},\n  {\"b\":2}\n]\n");
    assert_eq!(got, vec![r#"{"a":1}"#, r#"{"b":2}"#]);
}

#[test]
fn newline_delimited_messages_without_an_array_wrapper_also_decode() {
    // Some proxies rewrite a streamed array into NDJSON.
    let mut decoder = JsonStreamDecoder::new();
    let got = decoder.push(b"{\"a\":1}\n{\"b\":2}\n");
    assert_eq!(got, vec![r#"{"a":1}"#, r#"{"b":2}"#]);
}

#[test]
fn an_empty_array_yields_nothing_and_is_not_an_error() {
    let mut decoder = JsonStreamDecoder::new();
    assert!(decoder.push(b"[]").is_empty());
    assert_eq!(decoder.finish().expect("clean end"), None);
    assert!(decoder.is_closed());
}

#[test]
fn a_stream_that_ends_mid_message_is_an_error_not_a_short_result() {
    // The failure this decoder exists to catch: a dropped connection must not
    // look like a finished conversation.
    let mut decoder = JsonStreamDecoder::new();
    assert!(decoder.push(br#"[{"text":"partial"#).is_empty());
    let err = decoder.finish().expect_err("truncation must be reported");
    assert!(err.to_string().contains("mid-message"), "got {err}");
}

#[test]
fn a_stream_that_ends_inside_a_string_is_also_an_error() {
    let mut decoder = JsonStreamDecoder::new();
    assert!(decoder.push(br#"[{"text":"unterminated"#).is_empty());
    assert!(decoder.finish().is_err());
}

#[test]
fn messages_already_delivered_survive_a_later_truncation() {
    // A stream that dies after three replies must still yield those three.
    let mut decoder = JsonStreamDecoder::new();
    let got = decoder.push(br#"[{"a":1},{"b":2},{"c":3},{"d"#);
    assert_eq!(got.len(), 3, "got {got:?}");
    assert!(decoder.finish().is_err());
}

#[test]
fn a_trailing_bare_literal_is_terminated_by_the_end_of_the_stream() {
    // A bare number or `null` has no closing delimiter of its own.
    let mut decoder = JsonStreamDecoder::new();
    assert!(decoder.push(b"42").is_empty());
    assert_eq!(decoder.finish().expect("clean end"), Some("42".to_string()));
}

#[test]
fn a_top_level_string_message_is_delivered_whole() {
    let mut decoder = JsonStreamDecoder::new();
    let got = decoder.push(br#"["first","second"]"#);
    assert_eq!(got, vec![r#""first""#, r#""second""#]);
}

#[test]
fn an_element_that_is_itself_an_array_is_not_mistaken_for_the_wrapper() {
    let mut decoder = JsonStreamDecoder::new();
    let got = decoder.push(b"[[1,2],[3,4]]");
    assert_eq!(got, vec!["[1,2]", "[3,4]"]);
}

#[test]
fn every_delivered_message_parses_as_json() {
    // The contract callers rely on, asserted rather than assumed.
    let payload = r#"[{"a":{"b":[1,2,"}"]}},{"c":"\"quoted\""},{"d":null}]"#;
    for message in byte_at_a_time(payload) {
        serde_json::from_str::<serde_json::Value>(&message)
            .unwrap_or_else(|e| panic!("delivered a non-JSON message {message:?}: {e}"));
    }
}
