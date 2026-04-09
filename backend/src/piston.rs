use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct Judge0Request {
    language_id: u32,
    source_code: String,
    stdin: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu_time_limit: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wall_time_limit: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory_limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_network: Option<bool>,
}

#[derive(Deserialize)]
struct Judge0Response {
    stdout: Option<String>,
    stderr: Option<String>,
    compile_output: Option<String>,
    status: Judge0Status,
}

#[derive(Deserialize)]
struct Judge0Status {
    id: u32,
}

pub struct PistonRun {
    pub stdout: String,
    pub stderr: String,
    pub code: i64,
}

pub struct PistonClient {
    client: Client,
    base_url: String,
}

fn language_id(language: &str) -> Result<u32> {
    match language {
        "python3" => Ok(71),
        "javascript" => Ok(63),
        _ => anyhow::bail!("unsupported language: {}", language),
    }
}

impl PistonClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
        }
    }

    pub async fn run(&self, language: &str, code: &str, stdin: &str) -> Result<PistonRun> {
        let lang_id = language_id(language)?;
        let url = format!("{}/submissions?wait=true", self.base_url);
        let body = Judge0Request {
            language_id: lang_id,
            source_code: code.to_string(),
            stdin: stdin.to_string(),
            cpu_time_limit: Some(5.0),
            wall_time_limit: Some(10.0),
            memory_limit: Some(128_000),
            enable_network: Some(false),
        };

        let resp: Judge0Response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .timeout(std::time::Duration::from_secs(30))
            .send().await
            .context("judge0 request failed")?
            .error_for_status()
            .context("judge0 returned error status")?
            .json().await
            .context("judge0 response parse failed")?;

        // Judge0 status 13 = Internal Error (Judge0's fault, not the code's)
        if resp.status.id == 13 {
            anyhow::bail!("judge0 internal error");
        }

        // Status 3 = Accepted (ran successfully), anything else = failure
        let code = if resp.status.id == 3 { 0 } else { 1 };

        let stderr = [resp.stderr, resp.compile_output]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("\n");

        Ok(PistonRun {
            stdout: resp.stdout.unwrap_or_default(),
            stderr,
            code,
        })
    }
}

/// Dispatch to the correct language wrapper.
pub fn wrap_solution(language: &str, code: &str, input: &str) -> String {
    match language {
        "javascript" => wrap_solution_js(code),
        _ => wrap_solution_py(code, input),
    }
}

fn wrap_solution_py(solution_code: &str, _input: &str) -> String {
    format!(r#"
import json, sys

{solution_code}

# Harness
if __name__ == "__main__":
    lines = sys.stdin.read().strip().split('\n')
    args = [json.loads(line) for line in lines if line.strip()]
    s = Solution()
    for method in [m for m in dir(s) if not m.startswith('_')]:
        try:
            result = getattr(s, method)(*args)
            print(json.dumps(result))
            break
        except Exception as e:
            print(f"ERROR: {{e}}", file=sys.stderr)
"#, solution_code = solution_code)
}

fn wrap_solution_js(code: &str) -> String {
    let fn_name = extract_js_function_name(code).unwrap_or_else(|| "solution".to_string());

    format!(r#"
{code}

const lines = require('fs').readFileSync('/dev/stdin', 'utf8').trim().split('\n');
const args = lines.filter(l => l.trim()).map(JSON.parse);
const result = {fn_name}(...args);
console.log(JSON.stringify(result));
"#, code = code, fn_name = fn_name)
}

fn extract_js_function_name(code: &str) -> Option<String> {
    // Match: var/let/const name = function(
    let re1 = regex::Regex::new(r"(?:var|let|const)\s+(\w+)\s*=\s*function").ok()?;
    if let Some(cap) = re1.captures(code) {
        return Some(cap[1].to_string());
    }
    // Match: function name(
    let re2 = regex::Regex::new(r"function\s+(\w+)\s*\(").ok()?;
    if let Some(cap) = re2.captures(code) {
        return Some(cap[1].to_string());
    }
    // Match: var/let/const name = (...) =>
    let re3 = regex::Regex::new(r"(?:var|let|const)\s+(\w+)\s*=\s*\(").ok()?;
    if let Some(cap) = re3.captures(code) {
        return Some(cap[1].to_string());
    }
    None
}
