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

use cxas_core::{
    expand_path, method_spec, ApiVersion, AppRef, CoreError, Location, RequestBuilder, UrlError,
    DEFAULT_ENDPOINT, METHODS,
};
use std::collections::BTreeMap;

fn params(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn app() -> AppRef {
    AppRef::new("my-project", Location::new("us").unwrap(), "my-app").expect("app ref")
}

// ---------------------------------------------------------------- url

#[test]
fn reserved_expansion_keeps_resource_name_slashes() {
    // `{+parent}` is a resource name; its slashes are structural. Encoding them
    // yields a URL CES answers with 404 and nothing in the type system objects.
    let out = expand_path(
        "v1/{+parent}/agents",
        &params(&[("parent", "projects/p/locations/us/apps/a")]),
    )
    .expect("expand");
    assert_eq!(out, "v1/projects/p/locations/us/apps/a/agents");
}

#[test]
fn plain_expansion_encodes_slashes() {
    // Without `+` the value is one opaque segment, so a slash inside it must
    // not silently create a new path segment.
    let out = expand_path("v1/{id}:run", &params(&[("id", "a/b")])).expect("expand");
    assert_eq!(out, "v1/a%2Fb:run");
}

#[test]
fn reserved_expansion_still_encodes_unsafe_characters() {
    // Structure is preserved, but an id must not be able to smuggle a query
    // string or a fragment into the URL.
    let out = expand_path("v1/{+name}", &params(&[("name", "apps/a b?x=1")])).expect("expand");
    assert_eq!(out, "v1/apps/a%20b%3Fx%3D1");
}

#[test]
fn action_suffix_after_template_is_preserved() {
    let out = expand_path(
        "v1/{+name}:exportApp",
        &params(&[("name", "projects/p/locations/us/apps/a")]),
    )
    .expect("expand");
    assert_eq!(out, "v1/projects/p/locations/us/apps/a:exportApp");
}

#[test]
fn missing_parameter_is_named_in_the_error() {
    let err = expand_path("v1/{+parent}/agents", &params(&[])).unwrap_err();
    assert_eq!(err, UrlError::MissingParameter("parent".to_string()));
}

#[test]
fn empty_parameter_is_rejected() {
    // An empty value would collapse the path and silently address the parent.
    let err = expand_path("v1/{+name}", &params(&[("name", "")])).unwrap_err();
    assert_eq!(err, UrlError::EmptyParameter("name".to_string()));
}

#[test]
fn unclosed_brace_is_rejected() {
    let err = expand_path("v1/{+name", &params(&[("name", "x")])).unwrap_err();
    assert!(matches!(err, UrlError::UnclosedBrace(_)));
}

// ---------------------------------------------------------------- names

#[test]
fn app_name_embeds_project_and_location() {
    assert_eq!(app().name(), "projects/my-project/locations/us/apps/my-app");
}

#[test]
fn location_parent_stops_above_the_app() {
    assert_eq!(app().location_parent(), "projects/my-project/locations/us");
}

#[test]
fn child_names_hang_off_the_app() {
    assert_eq!(
        app().child("agents", "root"),
        "projects/my-project/locations/us/apps/my-app/agents/root"
    );
}

#[test]
fn a_resource_name_cannot_be_built_without_a_location() {
    // #401 is structurally unrepeatable: Location has no Default, and AppRef
    // has no constructor that omits it. The nearest mistake -- an empty
    // location string -- is refused at the Location boundary.
    assert!(matches!(Location::new(""), Err(CoreError::LocationRequired)));
    assert!(Location::new("__default_global__").is_err());
}

#[test]
fn empty_project_or_app_is_rejected() {
    let loc = Location::new("us").unwrap();
    assert!(AppRef::new("", loc.clone(), "a").is_err());
    assert!(AppRef::new("p", loc, "").is_err());
}

// ---------------------------------------------------------------- requests

#[test]
fn get_request_targets_the_documented_url() {
    let spec = method_spec("ces.projects.locations.apps.get", ApiVersion::V1).expect("spec");
    let req = RequestBuilder::default()
        .build(spec, &params(&[("name", &app().name())]), &BTreeMap::new(), None)
        .expect("build");

    assert_eq!(req.http_method, "GET");
    assert_eq!(
        req.url,
        format!("{DEFAULT_ENDPOINT}/v1/projects/my-project/locations/us/apps/my-app")
    );
    assert!(req.body.is_none());
}

#[test]
fn list_request_uses_the_location_parent() {
    let spec = method_spec("ces.projects.locations.apps.list", ApiVersion::V1).expect("spec");
    let req = RequestBuilder::default()
        .build(
            spec,
            &params(&[("parent", &app().location_parent())]),
            &BTreeMap::new(),
            None,
        )
        .expect("build");
    assert_eq!(
        req.url,
        format!("{DEFAULT_ENDPOINT}/v1/projects/my-project/locations/us/apps")
    );
}

#[test]
fn bearer_token_is_attached_when_configured() {
    let spec = method_spec("ces.projects.locations.apps.get", ApiVersion::V1).expect("spec");
    let req = RequestBuilder::default()
        .with_token("tok-123")
        .build(spec, &params(&[("name", &app().name())]), &BTreeMap::new(), None)
        .expect("build");
    assert!(req
        .headers
        .contains(&("authorization".to_string(), "Bearer tok-123".to_string())));
}

#[test]
fn no_authorization_header_without_a_token() {
    let spec = method_spec("ces.projects.locations.apps.get", ApiVersion::V1).expect("spec");
    let req = RequestBuilder::default()
        .build(spec, &params(&[("name", &app().name())]), &BTreeMap::new(), None)
        .expect("build");
    assert!(!req.headers.iter().any(|(k, _)| k == "authorization"));
}

#[test]
fn query_string_is_sorted_and_encoded() {
    let spec = method_spec("ces.projects.locations.apps.list", ApiVersion::V1).expect("spec");
    let req = RequestBuilder::default()
        .build(
            spec,
            &params(&[("parent", &app().location_parent())]),
            &params(&[("pageSize", "50"), ("filter", "a b")]),
            None,
        )
        .expect("build");
    assert!(
        req.url.ends_with("/apps?filter=a%20b&pageSize=50"),
        "got {}",
        req.url
    );
}

#[test]
fn body_sets_the_json_content_type() {
    let spec = method_spec("ces.projects.locations.apps.create", ApiVersion::V1).expect("spec");
    let req = RequestBuilder::default()
        .build(
            spec,
            &params(&[("parent", &app().location_parent())]),
            &BTreeMap::new(),
            Some("{\"displayName\":\"demo\"}".to_string()),
        )
        .expect("build");
    assert_eq!(req.http_method, "POST");
    assert!(req
        .headers
        .contains(&("content-type".to_string(), "application/json".to_string())));
}

#[test]
fn custom_endpoint_drops_a_trailing_slash() {
    let spec = method_spec("ces.projects.locations.apps.get", ApiVersion::V1).expect("spec");
    let req = RequestBuilder::new("http://127.0.0.1:8080/")
        .build(spec, &params(&[("name", &app().name())]), &BTreeMap::new(), None)
        .expect("build");
    assert!(req.url.starts_with("http://127.0.0.1:8080/v1/"), "got {}", req.url);
}

#[test]
fn a_missing_template_parameter_names_the_method() {
    let spec = method_spec("ces.projects.locations.apps.get", ApiVersion::V1).expect("spec");
    let err = RequestBuilder::default()
        .build(spec, &BTreeMap::new(), &BTreeMap::new(), None)
        .unwrap_err();
    assert!(
        format!("{err}").contains("ces.projects.locations.apps.get"),
        "got {err}"
    );
}

// ---------------------------------------------------------------- registry

#[test]
fn evaluation_methods_are_registered_only_on_v1beta() {
    // Evaluations do not exist on the v1 surface at all, so a v1 lookup must
    // fail rather than silently produce a URL CES will 404.
    assert!(method_spec("ces.projects.locations.apps.evaluations.list", ApiVersion::V1Beta).is_some());
    assert!(method_spec("ces.projects.locations.apps.evaluations.list", ApiVersion::V1).is_none());
}

#[test]
fn every_registered_path_starts_with_its_api_version() {
    for spec in METHODS {
        assert!(
            spec.path.starts_with(&format!("{}/", spec.api_version.as_str())),
            "{} path {} does not match version {}",
            spec.id,
            spec.path,
            spec.api_version.as_str()
        );
    }
}

#[test]
fn registry_has_no_duplicate_entries() {
    let mut seen = std::collections::HashSet::new();
    for spec in METHODS {
        assert!(
            seen.insert((spec.id, spec.api_version)),
            "{} registered twice for {:?}",
            spec.id,
            spec.api_version
        );
    }
}

#[test]
fn every_registered_path_is_location_scoped() {
    // Every CES resource lives under projects/*/locations/*; a template that
    // did not take a resource name could not carry a location at all.
    for spec in METHODS {
        assert!(
            spec.path.contains("{+"),
            "{} has no reserved-expansion parameter, so it cannot be location-scoped",
            spec.id
        );
    }
}
