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

//! Where a CES access token comes from, and when it stops being valid.
//!
//! Resolution is split from acquisition on purpose. [`resolve`] decides *which*
//! credential applies from an environment snapshot and does no I/O at all, so
//! the precedence rules -- the part that is easy to get wrong and impossible to
//! debug in production -- are testable without a filesystem, a network, or a
//! Google account. Only `TokenProvider::token` talks to anything.
//!
//! What is supported, and what is deliberately not:
//!
//! | Credential | Supported | Why |
//! |---|---|---|
//! | explicit `--oauth-token` | yes | no ambiguity to resolve |
//! | `CXAS_ACCESS_TOKEN` | yes | the CI escape hatch |
//! | ADC authorized user | yes | plain refresh-token grant, no signing |
//! | metadata server | yes | plain HTTP GET on GCE, Cloud Run, GKE |
//! | `gcloud auth print-access-token` | yes | the ordinary local-dev path |
//! | service-account key JSON | **no** | needs RS256 JWT signing |
//! | external / workload identity | **no** | needs an STS exchange |
//!
//! The last two return a typed error naming the credential and the supported
//! alternative. Silently falling through to a different credential is worse
//! than failing: the request would go out as the wrong principal and the
//! resulting 403 would point at the wrong problem.

use crate::CoreError;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Refresh this long before nominal expiry.
///
/// A token that expires mid-flight fails the request it was fetched for, so
/// the margin covers clock skew plus one slow round trip.
pub const REFRESH_SKEW: Duration = Duration::from_secs(60);

/// Assumed lifetime for a token whose source reports no expiry.
///
/// `gcloud auth print-access-token` prints a bare token with no metadata.
/// Google issues these with an hour of life; assuming less costs an occasional
/// extra subprocess and never serves an expired token.
pub const ASSUMED_LIFETIME: Duration = Duration::from_secs(45 * 60);

/// Environment variable holding a ready-made token.
pub const ACCESS_TOKEN_ENV: &str = "CXAS_ACCESS_TOKEN";
/// Standard Google variable pointing at a credential file.
pub const CREDENTIALS_ENV: &str = "GOOGLE_APPLICATION_CREDENTIALS";
/// Standard Google variable overriding the metadata server host.
pub const METADATA_HOST_ENV: &str = "GCE_METADATA_HOST";
/// Default metadata server host on Google compute platforms.
pub const DEFAULT_METADATA_HOST: &str = "metadata.google.internal";
/// Google's OAuth 2 token endpoint.
pub const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";

/// A refresh-token credential, as written by `gcloud auth application-default
/// login`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedUser {
    pub client_id: String,
    pub client_secret: String,
    pub refresh_token: String,
}

/// Which credential this process will use, decided before any I/O.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialSource {
    /// A token supplied verbatim; never refreshed because there is nothing to
    /// refresh it with.
    Static(String),
    AuthorizedUser(AuthorizedUser),
    /// The metadata server, at this host.
    MetadataServer(String),
    /// Shell out to `gcloud auth print-access-token`.
    GcloudCli,
}

impl CredentialSource {
    /// A short label for diagnostics. Never includes the credential itself.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Static(_) => "static-token",
            Self::AuthorizedUser(_) => "adc-authorized-user",
            Self::MetadataServer(_) => "metadata-server",
            Self::GcloudCli => "gcloud-cli",
        }
    }

    /// Whether this source can produce a fresh token after the current one
    /// expires. A static token cannot, so a long-running process holding one
    /// will eventually start failing and should say why.
    pub fn is_refreshable(&self) -> bool {
        !matches!(self, Self::Static(_))
    }
}

/// The parts of the outside world credential resolution reads.
///
/// A trait rather than direct `std::env` calls so the precedence tests do not
/// mutate global process state -- which they cannot do safely in parallel, and
/// which would make them depend on the developer's own gcloud login.
pub trait Host {
    fn var(&self, key: &str) -> Option<String>;
    fn read(&self, path: &Path) -> Option<String>;
    /// The well-known application-default credentials path for this platform.
    fn well_known_adc(&self) -> Option<PathBuf>;
    /// Whether this process appears to be running on Google compute.
    fn on_google_compute(&self) -> bool;
}

/// The real environment.
pub struct RealHost;

impl Host for RealHost {
    fn var(&self, key: &str) -> Option<String> {
        std::env::var(key).ok().filter(|v| !v.trim().is_empty())
    }

    fn read(&self, path: &Path) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }

    fn well_known_adc(&self) -> Option<PathBuf> {
        // gcloud writes under %APPDATA% on Windows and $HOME/.config elsewhere.
        // Hardcoding the POSIX path would make every Windows developer look
        // like they had never logged in.
        let base = if cfg!(windows) {
            PathBuf::from(self.var("APPDATA")?)
        } else {
            PathBuf::from(self.var("HOME")?).join(".config")
        };
        Some(base.join("gcloud").join("application_default_credentials.json"))
    }

    fn on_google_compute(&self) -> bool {
        // Only the explicit signals. Probing the metadata IP would add a
        // multi-second hang on every laptop that is not on GCE.
        self.var(METADATA_HOST_ENV).is_some()
            || self.var("K_SERVICE").is_some()
            || self.var("FUNCTION_TARGET").is_some()
            || self.var("CLOUD_RUN_JOB").is_some()
    }
}

/// What a credential JSON file declares itself to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdcKind {
    AuthorizedUser(AuthorizedUser),
    /// A service-account key. Named so the error can say which account.
    ServiceAccount { client_email: String },
    ExternalAccount { audience: String },
}

/// Parse a Google credential file.
///
/// Reports the credential *kind* even when it is unsupported, so the caller can
/// say "this is a service-account key, which needs signing" instead of "not
/// valid JSON".
pub fn parse_credential_file(contents: &str) -> Result<AdcKind, CoreError> {
    let value: serde_json::Value = serde_json::from_str(contents)
        .map_err(|e| CoreError::Auth(format!("credential file is not valid JSON: {e}")))?;

    let field = |key: &str| -> Option<String> {
        value
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .filter(|s| !s.is_empty())
    };

    match field("type").as_deref() {
        Some("authorized_user") => {
            let (Some(client_id), Some(client_secret), Some(refresh_token)) = (
                field("client_id"),
                field("client_secret"),
                field("refresh_token"),
            ) else {
                return Err(CoreError::Auth(
                    "authorized_user credential is missing client_id, client_secret, or refresh_token"
                        .to_string(),
                ));
            };
            Ok(AdcKind::AuthorizedUser(AuthorizedUser {
                client_id,
                client_secret,
                refresh_token,
            }))
        }
        Some("service_account") => Ok(AdcKind::ServiceAccount {
            client_email: field("client_email").unwrap_or_else(|| "<unnamed>".to_string()),
        }),
        Some("external_account") => Ok(AdcKind::ExternalAccount {
            audience: field("audience").unwrap_or_else(|| "<unnamed>".to_string()),
        }),
        Some(other) => Err(CoreError::Auth(format!(
            "unrecognised credential type {other:?}"
        ))),
        None => Err(CoreError::Auth(
            "credential file declares no \"type\" field".to_string(),
        )),
    }
}

/// Turn a parsed credential into a usable source, or explain why it is not one.
fn source_from(kind: AdcKind, origin: &str) -> Result<CredentialSource, CoreError> {
    match kind {
        AdcKind::AuthorizedUser(user) => Ok(CredentialSource::AuthorizedUser(user)),
        AdcKind::ServiceAccount { client_email } => Err(CoreError::Auth(format!(
            "{origin} holds a service-account key for {client_email}; signing a key file is not \
             implemented. Use `gcloud auth application-default login`, run on Google compute so \
             the metadata server can issue tokens, or pass an already-minted token in \
             {ACCESS_TOKEN_ENV}"
        ))),
        AdcKind::ExternalAccount { audience } => Err(CoreError::Auth(format!(
            "{origin} holds an external-account credential for {audience}; the STS token exchange \
             is not implemented. Pass an already-minted token in {ACCESS_TOKEN_ENV}"
        ))),
    }
}

/// Decide which credential to use. Performs no network I/O.
///
/// Precedence, highest first: an explicit token, `CXAS_ACCESS_TOKEN`,
/// `GOOGLE_APPLICATION_CREDENTIALS`, the well-known ADC file, the metadata
/// server, then `gcloud`. This mirrors Google's own client libraries, so a
/// machine already configured for `gcloud` behaves the same here.
///
/// An unusable credential at a *higher* precedence is an error, not a reason to
/// fall through: if `GOOGLE_APPLICATION_CREDENTIALS` points at a service-account
/// key, silently authenticating as the developer instead would send the request
/// as the wrong principal.
pub fn resolve(host: &dyn Host, explicit: Option<&str>) -> Result<CredentialSource, CoreError> {
    if let Some(token) = explicit.map(str::trim).filter(|t| !t.is_empty()) {
        return Ok(CredentialSource::Static(token.to_string()));
    }
    if let Some(token) = host.var(ACCESS_TOKEN_ENV) {
        return Ok(CredentialSource::Static(token));
    }

    if let Some(raw) = host.var(CREDENTIALS_ENV) {
        let path = PathBuf::from(&raw);
        let Some(contents) = host.read(&path) else {
            return Err(CoreError::Auth(format!(
                "{CREDENTIALS_ENV} points at {raw}, which cannot be read"
            )));
        };
        return source_from(parse_credential_file(&contents)?, &raw);
    }

    if let Some(path) = host.well_known_adc() {
        if let Some(contents) = host.read(&path) {
            return source_from(
                parse_credential_file(&contents)?,
                &path.display().to_string(),
            );
        }
    }

    if host.on_google_compute() {
        let hostname = host
            .var(METADATA_HOST_ENV)
            .unwrap_or_else(|| DEFAULT_METADATA_HOST.to_string());
        return Ok(CredentialSource::MetadataServer(hostname));
    }

    Ok(CredentialSource::GcloudCli)
}

/// A token and the moment it stops being usable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedToken {
    pub value: String,
    pub expires_at: SystemTime,
}

impl CachedToken {
    /// Build from a lifetime as reported by an OAuth endpoint.
    pub fn expiring_in(value: impl Into<String>, lifetime: Duration, now: SystemTime) -> Self {
        Self {
            value: value.into(),
            expires_at: now + lifetime,
        }
    }

    /// Usable at `now`, with the refresh margin already subtracted.
    ///
    /// `now` is a parameter rather than a call to `SystemTime::now` so expiry
    /// is testable without sleeping.
    pub fn is_fresh(&self, now: SystemTime) -> bool {
        match self.expires_at.duration_since(now) {
            Ok(remaining) => remaining > REFRESH_SKEW,
            Err(_) => false,
        }
    }
}

/// Read `access_token` and `expires_in` out of an OAuth token response.
///
/// Shared by the refresh-token grant and the metadata server, which return the
/// same shape.
pub fn parse_token_response(body: &str, now: SystemTime) -> Result<CachedToken, CoreError> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| CoreError::Auth(format!("token response is not valid JSON: {e}")))?;

    let Some(token) = value
        .get("access_token")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    else {
        // Surface the endpoint's own error text: "invalid_grant" means the
        // refresh token was revoked, which no retry will fix.
        let detail = value
            .get("error_description")
            .or_else(|| value.get("error"))
            .and_then(|v| v.as_str())
            .unwrap_or("no access_token in response");
        return Err(CoreError::Auth(format!("token endpoint refused: {detail}")));
    };

    let lifetime = value
        .get("expires_in")
        .and_then(|v| v.as_u64())
        .map(Duration::from_secs)
        .unwrap_or(ASSUMED_LIFETIME);

    Ok(CachedToken::expiring_in(token, lifetime, now))
}

/// The metadata server URL that issues a token for the default service account.
pub fn metadata_token_url(host: &str) -> String {
    let host = host.trim_end_matches('/');
    let base = if host.starts_with("http://") || host.starts_with("https://") {
        host.to_string()
    } else {
        format!("http://{host}")
    };
    format!("{base}/computeMetadata/v1/instance/service-accounts/default/token")
}

/// Form body for the refresh-token grant.
pub fn refresh_grant_body(user: &AuthorizedUser) -> String {
    let pairs = [
        ("client_id", user.client_id.as_str()),
        ("client_secret", user.client_secret.as_str()),
        ("refresh_token", user.refresh_token.as_str()),
        ("grant_type", "refresh_token"),
    ];
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", form_encode(k), form_encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn form_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

#[cfg(feature = "rest")]
pub use provider::TokenProvider;

#[cfg(feature = "rest")]
mod provider {
    use super::*;
    use std::sync::Mutex;

    /// Holds a credential and hands out valid tokens, refreshing as needed.
    ///
    /// The cache is behind a `Mutex` rather than taking `&mut self` so one
    /// provider can be shared by concurrent requests: without it, ten parallel
    /// calls on a cold cache would each mint their own token.
    pub struct TokenProvider {
        source: CredentialSource,
        endpoint: String,
        cached: Mutex<Option<CachedToken>>,
        http: reqwest::Client,
    }

    impl TokenProvider {
        /// Resolve a credential from the real environment.
        pub fn discover(explicit: Option<&str>) -> Result<Self, CoreError> {
            Self::from_source(resolve(&RealHost, explicit)?)
        }

        pub fn from_source(source: CredentialSource) -> Result<Self, CoreError> {
            Ok(Self {
                source,
                endpoint: TOKEN_ENDPOINT.to_string(),
                cached: Mutex::new(None),
                http: reqwest::Client::builder()
                    .build()
                    .map_err(|e| CoreError::Auth(format!("building HTTP client: {e}")))?,
            })
        }

        /// Point the refresh-token grant at another endpoint, for tests.
        pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
            self.endpoint = endpoint.into();
            self
        }

        pub fn source(&self) -> &CredentialSource {
            &self.source
        }

        /// A token valid now, minted or reused.
        pub async fn token(&self) -> Result<String, CoreError> {
            let now = SystemTime::now();
            if let Some(cached) = self.cached.lock().expect("token cache").as_ref() {
                if cached.is_fresh(now) {
                    return Ok(cached.value.clone());
                }
            }

            let fresh = self.mint(now).await?;
            let value = fresh.value.clone();
            *self.cached.lock().expect("token cache") = Some(fresh);
            Ok(value)
        }

        async fn mint(&self, now: SystemTime) -> Result<CachedToken, CoreError> {
            match &self.source {
                CredentialSource::Static(token) => {
                    // No expiry is knowable and there is nothing to refresh
                    // with. Treated as long-lived; if it has in fact expired,
                    // CES answers 401 and that is the honest signal.
                    Ok(CachedToken::expiring_in(
                        token.clone(),
                        ASSUMED_LIFETIME,
                        now,
                    ))
                }
                CredentialSource::AuthorizedUser(user) => {
                    let response = self
                        .http
                        .post(&self.endpoint)
                        .header("content-type", "application/x-www-form-urlencoded")
                        .body(refresh_grant_body(user))
                        .send()
                        .await
                        .map_err(|e| CoreError::Auth(format!("refreshing token: {e}")))?;
                    let text = response
                        .text()
                        .await
                        .map_err(|e| CoreError::Auth(format!("reading token response: {e}")))?;
                    parse_token_response(&text, now)
                }
                CredentialSource::MetadataServer(host) => {
                    let response = self
                        .http
                        .get(metadata_token_url(host))
                        .header("Metadata-Flavor", "Google")
                        .send()
                        .await
                        .map_err(|e| {
                            CoreError::Auth(format!("querying metadata server {host}: {e}"))
                        })?;
                    let text = response
                        .text()
                        .await
                        .map_err(|e| CoreError::Auth(format!("reading metadata response: {e}")))?;
                    parse_token_response(&text, now)
                }
                CredentialSource::GcloudCli => Self::gcloud_token(now),
            }
        }

        fn gcloud_token(now: SystemTime) -> Result<CachedToken, CoreError> {
            // `gcloud` is a batch script on Windows, so the bare name is not
            // executable there. Trying both is cheaper than teaching every
            // caller which platform it is on.
            let candidates: &[&str] = if cfg!(windows) {
                &["gcloud.cmd", "gcloud"]
            } else {
                &["gcloud"]
            };

            let mut last = String::from("no candidate was attempted");
            for binary in candidates {
                match std::process::Command::new(binary)
                    .args(["auth", "print-access-token"])
                    .output()
                {
                    Ok(output) if output.status.success() => {
                        let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        if token.is_empty() {
                            return Err(CoreError::Auth(
                                "gcloud auth print-access-token printed nothing".to_string(),
                            ));
                        }
                        return Ok(CachedToken::expiring_in(token, ASSUMED_LIFETIME, now));
                    }
                    Ok(output) => {
                        last = String::from_utf8_lossy(&output.stderr).trim().to_string();
                    }
                    Err(e) => last = e.to_string(),
                }
            }

            Err(CoreError::Auth(format!(
                "no credential found: no {ACCESS_TOKEN_ENV}, no application-default credentials, \
                 not on Google compute, and `gcloud auth print-access-token` failed ({last}). \
                 Run `gcloud auth application-default login`"
            )))
        }
    }
}
