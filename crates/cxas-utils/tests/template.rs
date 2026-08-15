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

use cxas_utils::{render_environment, TemplateValue};
use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn boolean_placeholder_renders_as_json_bool() {
    let mut vars = BTreeMap::new();
    vars.insert("FLAG".into(), TemplateValue::Bool(true));
    let out = render_environment(&json!({"voice": "{{FLAG}}"}), &vars).unwrap();
    assert_eq!(out["voice"], json!(true));
    assert!(out["voice"].is_boolean());
}

#[test]
fn invalid_bool_string_is_error() {
    let mut vars = BTreeMap::new();
    vars.insert("FLAG".into(), TemplateValue::String("maybe".into()));
    let err = render_environment(&json!({"voice": "{{FLAG|bool}}"}), &vars).unwrap_err();
    assert!(matches!(err, cxas_utils::UtilsError::InvalidBoolTemplate));
}

#[tokio::test]
async fn paginate_follows_tokens() {
    use cxas_utils::{paginate, Page};
    let mut calls = 0u8;
    let items = paginate(|token| {
        calls += 1;
        let token = token.cloned();
        async move {
            match token.as_deref() {
                None => Ok(Page {
                    items: vec![1, 2],
                    next_page_token: Some("n".into()),
                }),
                Some("n") => Ok(Page {
                    items: vec![3],
                    next_page_token: None,
                }),
                _ => unreachable!(),
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(items, vec![1, 2, 3]);
    assert_eq!(calls, 2);
}
