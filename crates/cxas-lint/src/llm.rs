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

use std::path::PathBuf;
use std::time::Duration;

use crate::diagnostic::{Diagnostic, Severity};
use crate::error::LintError;

const API_KEY_ENV: &str = "CXAS_GEMINI_API_KEY";
const PROMPT: &str = include_str!("../prompts/semantic_review.txt");

pub struct InstructionFile {
    pub path: PathBuf,
    pub body: String,
}

pub struct LlmLintClient {
    http: reqwest::Client,
    endpoint: reqwest::Url,
    api_key_env: &'static str,
}

impl LlmLintClient {
    pub fn new(endpoint: &str) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("reqwest client"),
            endpoint: endpoint.parse().expect("valid Gemini endpoint URL"),
            api_key_env: API_KEY_ENV,
        }
    }

    pub async fn lint_instructions(
        &self,
        files: &[InstructionFile],
    ) -> Result<Vec<Diagnostic>, LintError> {
        let api_key = std::env::var(self.api_key_env)
            .map_err(|_| LintError::MissingApiKey(self.api_key_env))?;

        let mut prompt = PROMPT.trim().to_string();
        for file in files {
            prompt.push_str("\n\n# ");
            prompt.push_str(&file.path.display().to_string());
            prompt.push('\n');
            prompt.push_str(&file.body);
        }

        let response = self
            .http
            .post(self.endpoint.clone())
            .header("x-goog-api-key", api_key)
            .json(&serde_json::json!({
                "contents": [{"parts": [{"text": prompt}]}]
            }))
            .send()
            .await
            .map_err(|err| LintError::Http {
                status: err.status().map(|s| s.as_u16()).unwrap_or(0),
                body: err.to_string(),
            })?;

        let status = response.status().as_u16();
        let body = response.text().await.map_err(|err| LintError::Http {
            status,
            body: err.to_string(),
        })?;
        if !(200..300).contains(&status) {
            return Err(LintError::Http { status, body });
        }
        parse_model_diagnostics(&body)
    }
}

fn parse_model_diagnostics(body: &str) -> Result<Vec<Diagnostic>, LintError> {
    let value: serde_json::Value =
        serde_json::from_str(body.trim()).map_err(|_| LintError::UnparseableModel)?;
    let items = value.as_array().ok_or(LintError::UnparseableModel)?;
    let mut diagnostics = Vec::with_capacity(items.len());
    for item in items {
        let severity = match item
            .get("severity")
            .and_then(|v| v.as_str())
            .unwrap_or("warning")
            .to_ascii_lowercase()
            .as_str()
        {
            "error" => Severity::Error,
            "info" => Severity::Info,
            _ => Severity::Warning,
        };
        diagnostics.push(Diagnostic {
            rule_id: "LLM-SEMANTIC".into(),
            severity,
            path: PathBuf::from(item.get("path").and_then(|v| v.as_str()).unwrap_or("")),
            message: item
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            fix: None,
        });
    }
    Ok(diagnostics)
}

#[cfg(any(test, feature = "llm"))]
pub async fn spawn_json_listener(body: &'static str) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind 127.0.0.1:0");
    let addr = listener.local_addr().expect("local_addr");
    let url = format!("http://{addr}/");
    let join = tokio::spawn(async move {
        let Ok((mut socket, _)) = listener.accept().await else {
            return;
        };
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut buf = vec![0u8; 8192];
        let _ = socket.read(&mut buf).await;
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = socket.write_all(resp.as_bytes()).await;
        let _ = socket.shutdown().await;
    });
    (url, join)
}
