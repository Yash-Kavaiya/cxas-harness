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

//! Token acquisition against a stub OAuth endpoint and metadata server.
//!
//! What matters here is the shape of what goes on the wire -- the grant type,
//! the `Metadata-Flavor` header Google requires, whether a second call mints a
//! second token -- none of which is observable from the returned string.

#![cfg(feature = "rest")]

mod support;

use cxas_core::auth::{AuthorizedUser, TokenProvider};
use cxas_core::{method_spec, ApiVersion, AppRef, CesHttpClient, CredentialSource, Location, RequestBuilder};
use std::collections::BTreeMap;
use support::{serve, Reply};

fn user() -> AuthorizedUser {
    AuthorizedUser {
        client_id: "id.apps.googleusercontent.com".into(),
        client_secret: "secret".into(),
        refresh_token: "1//refresh".into(),
    }
}

#[tokio::test]
async fn a_refresh_grant_asks_the_token_endpoint_the_way_google_expects() {
    let stub = serve(vec![Reply::ok(
        r#"{"access_token":"ya29.minted","expires_in":3599}"#,
    )]);
    let provider = TokenProvider::from_source(CredentialSource::AuthorizedUser(user()))
        .expect("provider")
        .with_endpoint(&stub.url);

    let token = provider.token().await.expect("mints a token");
    assert_eq!(token, "ya29.minted");

    let got = stub.next_request();
    assert!(got.request_line.starts_with("POST /"), "got {}", got.request_line);
    assert!(
        got.has_header("content-type", "application/x-www-form-urlencoded"),
        "the token endpoint rejects anything else: {:?}",
        got.headers
    );
    assert!(got.body.contains("grant_type=refresh_token"), "got {}", got.body);
    assert!(got.body.contains("refresh_token=1%2F%2Frefresh"), "got {}", got.body);
}

#[tokio::test]
async fn a_valid_token_is_reused_instead_of_reminted() {
    // Minting per request would multiply every CES call by a round trip to
    // Google's token endpoint, and eventually trip its own rate limit.
    let stub = serve(vec![Reply::ok(
        r#"{"access_token":"ya29.minted","expires_in":3599}"#,
    )]);
    let provider = TokenProvider::from_source(CredentialSource::AuthorizedUser(user()))
        .expect("provider")
        .with_endpoint(&stub.url);

    let first = provider.token().await.expect("first");
    let second = provider.token().await.expect("second");
    assert_eq!(first, second);

    stub.next_request();
    assert!(
        stub.saw_no_further_request(),
        "the second call minted a second token instead of reusing the first"
    );
}

#[tokio::test]
async fn an_almost_expired_token_is_replaced_rather_than_reused() {
    // `expires_in` inside the refresh skew means the token would die in flight.
    let stub = serve(vec![
        Reply::ok(r#"{"access_token":"first","expires_in":5}"#),
        Reply::ok(r#"{"access_token":"second","expires_in":3599}"#),
    ]);
    let provider = TokenProvider::from_source(CredentialSource::AuthorizedUser(user()))
        .expect("provider")
        .with_endpoint(&stub.url);

    assert_eq!(provider.token().await.expect("first"), "first");
    assert_eq!(
        provider.token().await.expect("second"),
        "second",
        "a token expiring inside the skew must be refreshed"
    );
}

#[tokio::test]
async fn the_metadata_server_is_asked_with_the_header_it_requires() {
    // Google's metadata server refuses requests without `Metadata-Flavor`,
    // specifically so a confused browser cannot exfiltrate a token.
    let stub = serve(vec![Reply::ok(
        r#"{"access_token":"ya29.metadata","expires_in":3599}"#,
    )]);
    let host = stub.url.trim_start_matches("http://").to_string();
    let provider =
        TokenProvider::from_source(CredentialSource::MetadataServer(host)).expect("provider");

    assert_eq!(provider.token().await.expect("token"), "ya29.metadata");

    let got = stub.next_request();
    assert_eq!(
        got.request_line,
        "GET /computeMetadata/v1/instance/service-accounts/default/token HTTP/1.1"
    );
    assert!(
        got.has_header("metadata-flavor", "Google"),
        "missing Metadata-Flavor: {:?}",
        got.headers
    );
}

#[tokio::test]
async fn a_revoked_refresh_token_reports_the_reason_not_a_retry() {
    let stub = serve(vec![Reply::status(
        400,
        r#"{"error":"invalid_grant","error_description":"Token has been expired or revoked."}"#,
    )]);
    let provider = TokenProvider::from_source(CredentialSource::AuthorizedUser(user()))
        .expect("provider")
        .with_endpoint(&stub.url);

    let err = provider.token().await.expect_err("a revoked grant must fail");
    assert!(err.to_string().contains("revoked"), "got {err}");
}

#[tokio::test]
async fn a_static_token_needs_no_network_at_all() {
    let provider =
        TokenProvider::from_source(CredentialSource::Static("preset".into())).expect("provider");
    // No stub is running; reaching the network here would fail.
    assert_eq!(provider.token().await.expect("token"), "preset");
}

#[tokio::test]
async fn a_minted_token_reaches_ces_exactly_once() {
    // Two authorization headers is not a hypothetical: the builder carries one
    // and the provider adds another, and CES rejects the request rather than
    // picking a winner.
    let auth = serve(vec![Reply::ok(
        r#"{"access_token":"ya29.minted","expires_in":3599}"#,
    )]);
    let ces = serve(vec![Reply::ok(r#"{"name":"apps/demo"}"#)]);

    let provider = TokenProvider::from_source(CredentialSource::AuthorizedUser(user()))
        .expect("provider")
        .with_endpoint(&auth.url);
    let client = CesHttpClient::with_builder(RequestBuilder::new(&ces.url).with_token("stale"))
        .expect("client")
        .with_tokens(provider);

    let app = AppRef::new("proj", Location::new("us-central1").unwrap(), "demo").expect("app");
    let spec = method_spec("ces.projects.locations.apps.get", ApiVersion::V1).expect("spec");
    let params: BTreeMap<String, String> = [("name".to_string(), app.name())].into_iter().collect();

    client
        .call(spec, &params, &BTreeMap::new(), None)
        .await
        .expect("call must succeed");

    auth.next_request();
    let got = ces.next_request();
    let authorizations: Vec<&String> = got
        .headers
        .iter()
        .filter(|h| h.to_ascii_lowercase().starts_with("authorization:"))
        .collect();
    assert_eq!(
        authorizations.len(),
        1,
        "expected exactly one authorization header, got {authorizations:?}"
    );
    assert!(
        authorizations[0].contains("ya29.minted"),
        "the builder's stale token won: {authorizations:?}"
    );
}
