use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct Judge0Request {
    language_id: u32,
    source_code: String,
    stdin: String,
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

impl PistonClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
        }
    }

    pub async fn run_python(&self, code: &str, stdin: &str) -> Result<PistonRun> {
        let url = format!("{}/submissions?wait=true", self.base_url);
        let body = Judge0Request {
            language_id: 71, // Python 3
            source_code: code.to_string(),
            stdin: stdin.to_string(),
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

pub fn wrap_solution(solution_code: &str, _input: &str) -> String {
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
