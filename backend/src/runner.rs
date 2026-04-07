use crate::{
    config::Config,
    models::{Model, Problem, RaceEvent, RaceResult, RaceStatus, TestCase},
    piston::{PistonClient, wrap_solution},
};
use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};
use std::{sync::Arc, time::Instant};
use tokio::sync::broadcast;
use uuid::Uuid;
use chrono::Utc;

pub type EventSender = broadcast::Sender<RaceEvent>;

pub struct Runner {
    config: Arc<Config>,
    piston: Arc<PistonClient>,
    http: Client,
}

impl Runner {
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            piston: Arc::new(PistonClient::new(&config.piston_url)),
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap(),
            config,
        }
    }

    /// Race all active models on a problem. Emits SSE events as each finishes.
    pub async fn race(
        &self,
        race_id: &str,
        problem: &Problem,
        models: Vec<Model>,
        tx: EventSender,
    ) -> Vec<RaceResult> {
        let test_cases: Vec<TestCase> = serde_json::from_str(&problem.test_cases)
            .unwrap_or_default();

        let mut handles = vec![];

        for model in models.into_iter().filter(|m| m.is_active && !m.is_human) {
            let runner = self.clone_cheap();
            let problem = problem.clone();
            let test_cases = test_cases.clone();
            let tx = tx.clone();
            let race_id = race_id.to_string();

            handles.push(tokio::spawn(async move {
                let _ = tx.send(RaceEvent {
                    race_id: race_id.clone(),
                    model_id: model.id.clone(),
                    display_name: model.display_name.clone(),
                    status: RaceStatus::Running,
                    time_ms: None,
                    attempts: 0,
                });

                runner.race_one(&race_id, &model, &problem, &test_cases, &tx).await
            }));
        }

        let mut results = vec![];
        for handle in handles {
            if let Ok(result) = handle.await {
                results.push(result);
            }
        }
        results
    }

    async fn race_one(
        &self,
        race_id: &str,
        model: &Model,
        problem: &Problem,
        test_cases: &[TestCase],
        tx: &EventSender,
    ) -> RaceResult {
        let start = Instant::now();
        let api_key = std::env::var(&model.api_key_env).unwrap_or_default();
        let mut attempts = 0;
        let mut solved = false;
        let mut last_error = String::new();

        for attempt in 1..=3 {
            attempts = attempt;
            let prompt = if attempt == 1 {
                build_prompt(&problem.title, &problem.description, &problem.starter_code)
            } else {
                build_retry_prompt(&problem.title, &problem.description, &problem.starter_code, &last_error)
            };

            let code = match self.call_model(model, &api_key, &prompt).await {
                Ok(c) => c,
                Err(e) => { last_error = e.to_string(); continue; }
            };

            match self.verify(&code, test_cases).await {
                Ok(true) => { solved = true; break; }
                Ok(false) => { last_error = "wrong answer".into(); }
                Err(e) => { last_error = e.to_string(); }
            }
        }

        let elapsed_ms = start.elapsed().as_millis() as i64;
        let status = if solved { RaceStatus::Solved } else { RaceStatus::Failed };

        let _ = tx.send(RaceEvent {
            race_id: race_id.to_string(),
            model_id: model.id.clone(),
            display_name: model.display_name.clone(),
            status,
            time_ms: if solved { Some(elapsed_ms) } else { None },
            attempts: attempts as i64,
        });

        RaceResult {
            id: Uuid::new_v4().to_string(),
            problem_id: problem.id.clone(),
            model_id: model.id.clone(),
            solved,
            time_ms: if solved { Some(elapsed_ms) } else { None },
            attempts: attempts as i64,
            run_at: Utc::now().to_rfc3339(),
        }
    }

    async fn call_model(&self, model: &Model, api_key: &str, prompt: &str) -> Result<String> {
        let (url, body) = build_api_request(&model.provider, &model.name, api_key, prompt)?;
        let auth_value = auth_header_value(&model.provider, api_key);
        let resp: Value = self.http
            .post(&url)
            .header("Content-Type", "application/json")
            .header(auth_header_name(&model.provider), auth_value)
            .json(&body)
            .send().await?
            .json().await?;
        extract_code(&resp, &model.provider)
    }

    async fn verify(&self, code: &str, test_cases: &[TestCase]) -> Result<bool> {
        if test_cases.is_empty() {
            return Ok(true);
        }
        for tc in test_cases {
            let wrapped = wrap_solution(code, &tc.input);
            let run = self.piston.run_python(&wrapped, &tc.input).await?;
            if run.code != 0 { return Ok(false); }
            if !tc.expected_output.is_empty() {
                let got = run.stdout.trim();
                let want = tc.expected_output.trim();
                if got != want { return Ok(false); }
            }
        }
        Ok(true)
    }

    fn clone_cheap(&self) -> Self {
        Self {
            config: self.config.clone(),
            piston: self.piston.clone(),
            http: self.http.clone(),
        }
    }
}

fn build_prompt(title: &str, description: &str, starter: &str) -> String {
    format!(
        "Solve the following LeetCode problem in Python. Return only the solution class/function, no explanation.\n\n{}\n\n{}\n\n{}",
        title, description, starter
    )
}

fn build_retry_prompt(title: &str, description: &str, starter: &str, error: &str) -> String {
    format!(
        "Solve the following LeetCode problem in Python. Return only the solution class/function, no explanation.\nYour previous attempt failed with: {}\n\n{}\n\n{}\n\n{}",
        error, title, description, starter
    )
}

fn build_api_request(provider: &str, model_name: &str, _api_key: &str, prompt: &str) -> Result<(String, Value)> {
    match provider {
        "openai" | "xai" | "fireworks" | "deepseek" | "mistral" | "groq" | "github" => {
            let base = match provider {
                "openai" => "https://api.openai.com/v1",
                "xai" => "https://api.x.ai/v1",
                "fireworks" => "https://api.fireworks.ai/inference/v1",
                "deepseek" => "https://api.deepseek.com/v1",
                "mistral" => "https://api.mistral.ai/v1",
                "groq" => "https://api.groq.com/openai/v1",
                "github" => "https://models.inference.ai.azure.com",
                _ => unreachable!(),
            };
            Ok((
                format!("{}/chat/completions", base),
                json!({ "model": model_name, "messages": [{"role":"user","content": prompt}], "max_tokens": 2048 })
            ))
        }
        "anthropic" => Ok((
            "https://api.anthropic.com/v1/messages".into(),
            json!({ "model": model_name, "max_tokens": 2048, "messages": [{"role":"user","content": prompt}] })
        )),
        "google" => Ok((
            format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent", model_name),
            json!({ "contents": [{"parts": [{"text": prompt}]}] })
        )),
        "cloudflare" => {
            let account_id = std::env::var("CF_ACCOUNT_ID").unwrap_or_default();
            Ok((
                format!(
                    "https://api.cloudflare.com/client/v4/accounts/{}/ai/run/{}",
                    account_id, model_name
                ),
                json!({ "messages": [{"role":"user","content": prompt}], "max_tokens": 2048 })
            ))
        },
        "qwen" => Ok((
            "https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation".into(),
            json!({ "model": model_name, "input": { "messages": [{"role":"user","content": prompt}] } })
        )),
        "moonshot" => Ok((
            "https://api.moonshot.cn/v1/chat/completions".into(),
            json!({ "model": model_name, "messages": [{"role":"user","content": prompt}] })
        )),
        "doubao" => Ok((
            "https://ark.cn-beijing.volces.com/api/v3/chat/completions".into(),
            json!({ "model": model_name, "messages": [{"role":"user","content": prompt}] })
        )),
        "hunyuan" => Ok((
            "https://hunyuan.tencentcloudapi.com/".into(),
            json!({ "Model": model_name, "Messages": [{"Role":"user","Content": prompt}] })
        )),
        p => anyhow::bail!("unknown provider: {}", p),
    }
}

/// Returns the header name to use for authentication.
fn auth_header_name(provider: &str) -> &'static str {
    match provider {
        "anthropic" => "x-api-key",
        "google" => "x-goog-api-key",
        _ => "Authorization",
    }
}

/// Returns the header value to use for authentication.
/// OpenAI-compatible providers expect `Bearer <key>`; others use the raw key.
fn auth_header_value(provider: &str, api_key: &str) -> String {
    match provider {
        "anthropic" | "google" => api_key.to_string(),
        _ => format!("Bearer {}", api_key),
    }
}

fn extract_code(resp: &Value, provider: &str) -> Result<String> {
    let text = match provider {
        "anthropic" => resp["content"][0]["text"].as_str().unwrap_or(""),
        "google" => resp["candidates"][0]["content"]["parts"][0]["text"].as_str().unwrap_or(""),
        "qwen" => resp["output"]["text"].as_str().unwrap_or(""),
        "hunyuan" => resp["Choices"][0]["Message"]["Content"].as_str().unwrap_or(""),
        "cloudflare" => resp["result"]["response"].as_str().unwrap_or(""),
        _ => resp["choices"][0]["message"]["content"].as_str().unwrap_or(""),
    };

    if let Some(start) = text.find("```python") {
        let rest = &text[start + 9..];
        if let Some(end) = rest.find("```") {
            return Ok(rest[..end].trim().to_string());
        }
    }
    if let Some(start) = text.find("```") {
        let rest = &text[start + 3..];
        if let Some(end) = rest.find("```") {
            return Ok(rest[..end].trim().to_string());
        }
    }
    Ok(text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_api_request_groq() {
        let (url, body) = build_api_request("groq", "llama-3.3-70b-versatile", "", "test").unwrap();
        assert_eq!(url, "https://api.groq.com/openai/v1/chat/completions");
        assert_eq!(body["model"], "llama-3.3-70b-versatile");
        assert!(body["messages"].is_array());
    }

    #[test]
    fn test_build_api_request_github() {
        let (url, body) = build_api_request("github", "gpt-4o-mini", "", "test").unwrap();
        assert_eq!(url, "https://models.inference.ai.azure.com/chat/completions");
        assert_eq!(body["model"], "gpt-4o-mini");
    }

    #[test]
    fn test_build_api_request_cloudflare() {
        std::env::set_var("CF_ACCOUNT_ID", "abc123");
        let (url, _) = build_api_request("cloudflare", "@cf/meta/llama-3.1-8b-instruct", "", "test").unwrap();
        assert!(url.contains("abc123"), "URL should contain account ID");
        assert!(url.contains("llama-3.1-8b-instruct"), "URL should contain model name");
    }

    #[test]
    fn test_build_api_request_unknown_provider_errors() {
        let result = build_api_request("notreal", "model", "", "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_code_cloudflare() {
        let resp = serde_json::json!({
            "result": { "response": "def solution():\n    return 42" },
            "success": true
        });
        let code = extract_code(&resp, "cloudflare").unwrap();
        assert_eq!(code, "def solution():\n    return 42");
    }

    #[test]
    fn test_extract_code_cloudflare_with_code_fence() {
        let resp = serde_json::json!({
            "result": { "response": "```python\ndef solution():\n    return 42\n```" }
        });
        let code = extract_code(&resp, "cloudflare").unwrap();
        assert_eq!(code, "def solution():\n    return 42");
    }

    #[test]
    fn test_extract_code_cloudflare_missing_field() {
        let resp = serde_json::json!({ "result": {} });
        let code = extract_code(&resp, "cloudflare").unwrap();
        assert_eq!(code, "");
    }
}
