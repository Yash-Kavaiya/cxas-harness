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

use crate::UtilsError;
use std::future::Future;

/// One page of results from a token-paginated API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_page_token: Option<String>,
}

/// Fetch every page by following `next_page_token` until it is `None`.
///
/// The fetch callback receives the previous page token (`None` on the first call).
pub async fn paginate<F, Fut, T>(mut fetch: F) -> Result<Vec<T>, UtilsError>
where
    F: FnMut(Option<&String>) -> Fut,
    Fut: Future<Output = Result<Page<T>, UtilsError>>,
{
    let mut all = Vec::new();
    let mut next_token: Option<String> = None;
    loop {
        let page = fetch(next_token.as_ref()).await?;
        all.extend(page.items);
        match page.next_page_token {
            Some(token) => next_token = Some(token),
            None => break,
        }
    }
    Ok(all)
}
