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

//! Credential precedence, without touching the machine running the tests.
//!
//! Every case here goes through a fake host, so the results do not depend on
//! whether the developer happens to be logged into gcloud, and the tests can
//! run in parallel without fighting over process environment.

use cxas_core::auth::{
    metadata_token_url, parse_credential_file, parse_token_response, refresh_grant_body,
    AuthorizedUser, CachedToken, Host, ACCESS_TOKEN_ENV, ASSUMED_LIFETIME, CREDENTIALS_ENV,
    DEFAULT_METADATA_HOST, METADATA_HOST_ENV, REFRESH_SKEW,
};
use cxas_core::{resolve_credential, AdcKind, CredentialSource};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

#[derive(Default)]
struct FakeHost {
    vars: BTreeMap<String, String>,
    files: BTreeMap<PathBuf, String>,
    well_known: Option<PathBuf>,
    on_compute: bool,
}

impl FakeHost {
    fn var(mut self, key: &str, value: &str) -> Self {
        self.vars.insert(key.to_string(), value.to_string());
        self
    }

    fn file(mut self, path: &str, contents: &str) -> Self {
        self.files.insert(PathBuf::from(path), contents.to_string());
        self
    }

    fn well_known(mut self, path: &str) -> Self {
        self.well_known = Some(PathBuf::from(path));
        self
    }

    fn on_compute(mut self) -> Self {
        self.on_compute = true;
        self
    }
}

impl Host for FakeHost {
    fn var(&self, key: &str) -> Option<String> {
        self.vars.get(key).cloned()
    }

    fn read(&self, path: &Path) -> Option<String> {
        self.files.get(path).cloned()
    }

    fn well_known_adc(&self) -> Option<PathBuf> {
        self.well_known.clone()
    }

    fn on_google_compute(&self) -> bool {
        self.on_compute
    }
}

const AUTHORIZED_USER: &str = r#"{
  "type": "authorized_user",
  "client_id": "id.apps.googleusercontent.com",
  "client_secret": "secret",
  "refresh_token": "1//refresh"
}"#;

const SERVICE_ACCOUNT: &str = r#"{
  "type": "service_account",
  "client_email": "robot@project.iam.gserviceaccount.com",
  "private_key": "-----BEGIN PRIVATE KEY-----"
}"#;

fn epoch(secs: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
}

#[test]
fn an_explicit_token_outranks_every_ambient_credential() {
    let host = FakeHost::default()
        .var(ACCESS_TOKEN_ENV, "from-env")
        .var(CREDENTIALS_ENV, "/adc.json")
        .file("/adc.json", AUTHORIZED_USER)
        .on_compute();

    let source = resolve_credential(&host, Some("explicit")).expect("resolves");
    assert_eq!(source, CredentialSource::Static("explicit".to_string()));
}

#[test]
fn a_blank_explicit_token_is_not_a_credential() {
    // `--oauth-token ""` from a CI template with an unset variable must not be
    // taken as an instruction to authenticate as nobody.
    let host = FakeHost::default().var(ACCESS_TOKEN_ENV, "from-env");
    let source = resolve_credential(&host, Some("   ")).expect("resolves");
    assert_eq!(source, CredentialSource::Static("from-env".to_string()));
}

#[test]
fn the_env_token_outranks_a_credential_file() {
    let host = FakeHost::default()
        .var(ACCESS_TOKEN_ENV, "from-env")
        .var(CREDENTIALS_ENV, "/adc.json")
        .file("/adc.json", AUTHORIZED_USER);

    assert_eq!(
        resolve_credential(&host, None).expect("resolves"),
        CredentialSource::Static("from-env".to_string())
    );
}

#[test]
fn an_authorized_user_file_becomes_a_refreshable_source() {
    let host = FakeHost::default()
        .var(CREDENTIALS_ENV, "/adc.json")
        .file("/adc.json", AUTHORIZED_USER);

    let source = resolve_credential(&host, None).expect("resolves");
    assert_eq!(
        source,
        CredentialSource::AuthorizedUser(AuthorizedUser {
            client_id: "id.apps.googleusercontent.com".to_string(),
            client_secret: "secret".to_string(),
            refresh_token: "1//refresh".to_string(),
        })
    );
    assert!(source.is_refreshable());
}

#[test]
fn a_service_account_key_fails_loudly_instead_of_falling_through() {
    // Falling back to gcloud here would authenticate as the developer while the
    // operator believed they were acting as the robot. The 403 that follows
    // points at the wrong problem entirely.
    let host = FakeHost::default()
        .var(CREDENTIALS_ENV, "/key.json")
        .file("/key.json", SERVICE_ACCOUNT)
        .on_compute();

    let err = resolve_credential(&host, None).expect_err("must not fall through");
    let msg = err.to_string();
    assert!(
        msg.contains("robot@project.iam.gserviceaccount.com"),
        "the error must name the account it refused: {msg}"
    );
    assert!(
        msg.contains("gcloud auth application-default login"),
        "the error must name a supported alternative: {msg}"
    );
}

#[test]
fn an_external_account_credential_names_its_audience() {
    let host = FakeHost::default().var(CREDENTIALS_ENV, "/wif.json").file(
        "/wif.json",
        r#"{"type":"external_account","audience":"//iam.googleapis.com/pool"}"#,
    );

    let err = resolve_credential(&host, None).expect_err("unsupported");
    assert!(
        err.to_string().contains("//iam.googleapis.com/pool"),
        "got {err}"
    );
}

#[test]
fn an_unreadable_credentials_path_is_an_error_not_a_fallback() {
    let host = FakeHost::default()
        .var(CREDENTIALS_ENV, "/missing.json")
        .on_compute();

    let err = resolve_credential(&host, None).expect_err("must not fall through");
    assert!(err.to_string().contains("/missing.json"), "got {err}");
}

#[test]
fn the_well_known_adc_file_is_used_when_no_variable_points_anywhere() {
    let host = FakeHost::default()
        .well_known("/home/.config/gcloud/application_default_credentials.json")
        .file(
            "/home/.config/gcloud/application_default_credentials.json",
            AUTHORIZED_USER,
        );

    assert!(matches!(
        resolve_credential(&host, None).expect("resolves"),
        CredentialSource::AuthorizedUser(_)
    ));
}

#[test]
fn google_compute_falls_to_the_metadata_server() {
    let host = FakeHost::default().on_compute();
    assert_eq!(
        resolve_credential(&host, None).expect("resolves"),
        CredentialSource::MetadataServer(DEFAULT_METADATA_HOST.to_string())
    );
}

#[test]
fn the_metadata_host_override_is_honoured() {
    let host = FakeHost::default()
        .on_compute()
        .var(METADATA_HOST_ENV, "127.0.0.1:8080");
    assert_eq!(
        resolve_credential(&host, None).expect("resolves"),
        CredentialSource::MetadataServer("127.0.0.1:8080".to_string())
    );
}

#[test]
fn a_bare_workstation_falls_back_to_the_gcloud_cli() {
    let host = FakeHost::default();
    assert_eq!(
        resolve_credential(&host, None).expect("resolves"),
        CredentialSource::GcloudCli
    );
}

#[test]
fn a_source_label_never_leaks_the_credential() {
    // Labels are printed in diagnostics and copied into bug reports.
    let secret = "ya29.super-secret-token";
    let source = CredentialSource::Static(secret.to_string());
    assert!(!source.label().contains(secret));

    let user = CredentialSource::AuthorizedUser(AuthorizedUser {
        client_id: "id".into(),
        client_secret: "shh".into(),
        refresh_token: "1//refresh".into(),
    });
    assert!(!user.label().contains("shh"));
    assert!(!user.label().contains("1//refresh"));
}

#[test]
fn a_static_token_is_not_refreshable() {
    assert!(!CredentialSource::Static("t".into()).is_refreshable());
    assert!(CredentialSource::GcloudCli.is_refreshable());
}

#[test]
fn a_credential_file_without_a_type_is_rejected_by_name() {
    let err = parse_credential_file(r#"{"client_id":"x"}"#).expect_err("no type");
    assert!(err.to_string().contains("type"), "got {err}");
}

#[test]
fn an_authorized_user_missing_its_refresh_token_is_rejected() {
    // Half a credential produces a refresh request that can only ever fail.
    let err = parse_credential_file(r#"{"type":"authorized_user","client_id":"x"}"#)
        .expect_err("incomplete");
    assert!(err.to_string().contains("refresh_token"), "got {err}");
}

#[test]
fn a_service_account_without_an_email_still_parses_as_one() {
    let kind = parse_credential_file(r#"{"type":"service_account"}"#).expect("kind is known");
    assert!(matches!(kind, AdcKind::ServiceAccount { .. }));
}

#[test]
fn a_token_inside_the_refresh_skew_is_already_stale() {
    let now = epoch(1_000);
    let token = CachedToken {
        value: "t".into(),
        expires_at: now + REFRESH_SKEW - Duration::from_secs(1),
    };
    assert!(
        !token.is_fresh(now),
        "a token expiring inside the skew must be refreshed, not used"
    );

    let comfortable = CachedToken {
        value: "t".into(),
        expires_at: now + REFRESH_SKEW + Duration::from_secs(1),
    };
    assert!(comfortable.is_fresh(now));
}

#[test]
fn an_expired_token_is_stale_rather_than_panicking() {
    // `duration_since` errors when the target is in the past; treating that as
    // "fresh" would serve an expired token forever.
    let now = epoch(1_000);
    let token = CachedToken {
        value: "t".into(),
        expires_at: epoch(900),
    };
    assert!(!token.is_fresh(now));
}

#[test]
fn a_token_response_carries_its_own_expiry() {
    let now = epoch(1_000);
    let token = parse_token_response(r#"{"access_token":"ya29.x","expires_in":3600}"#, now)
        .expect("parses");
    assert_eq!(token.value, "ya29.x");
    assert_eq!(token.expires_at, now + Duration::from_secs(3600));
}

#[test]
fn a_token_response_without_an_expiry_gets_the_conservative_default() {
    let now = epoch(1_000);
    let token = parse_token_response(r#"{"access_token":"ya29.x"}"#, now).expect("parses");
    assert_eq!(token.expires_at, now + ASSUMED_LIFETIME);
}

#[test]
fn a_refused_grant_surfaces_the_endpoints_own_reason() {
    // "invalid_grant" means the refresh token was revoked. Retrying cannot fix
    // it, so the message has to say so rather than read as a network blip.
    let err = parse_token_response(
        r#"{"error":"invalid_grant","error_description":"Token has been expired or revoked."}"#,
        epoch(1_000),
    )
    .expect_err("no token");
    assert!(
        err.to_string().contains("revoked"),
        "the reason must survive: {err}"
    );
}

#[test]
fn an_empty_access_token_is_not_a_token() {
    let err = parse_token_response(r#"{"access_token":""}"#, epoch(1)).expect_err("empty");
    assert!(err.to_string().contains("refused"), "got {err}");
}

#[test]
fn the_metadata_url_accepts_a_bare_host_or_a_full_origin() {
    assert_eq!(
        metadata_token_url("metadata.google.internal"),
        "http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token"
    );
    assert_eq!(
        metadata_token_url("http://127.0.0.1:8080/"),
        "http://127.0.0.1:8080/computeMetadata/v1/instance/service-accounts/default/token"
    );
}

#[test]
fn the_refresh_body_encodes_every_field_it_sends() {
    let body = refresh_grant_body(&AuthorizedUser {
        client_id: "id.apps.googleusercontent.com".into(),
        client_secret: "a+b/c".into(),
        refresh_token: "1//token".into(),
    });

    assert!(body.contains("grant_type=refresh_token"), "got {body}");
    // An unencoded `+` is read as a space by the token endpoint, which turns a
    // valid secret into a silent authentication failure.
    assert!(body.contains("client_secret=a%2Bb%2Fc"), "got {body}");
    assert!(body.contains("refresh_token=1%2F%2Ftoken"), "got {body}");
}
