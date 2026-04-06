use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct PistonRequest {
    language: String,
    version: String,
    files: Vec<PistonFile>,
    stdin: String,
}

#[derive(Serialize)]
struct PistonFile {
    name: String,
    content: String,
}

#[derive(Deserialize)]
pub struct PistonResponse {
    pub run: PistonRun,
}

#[derive(Deserialize)]
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
        let url = format!("{}/execute", self.base_url);
        let body = PistonRequest {
            language: "python".into(),
            version: "3.10.0".into(),
            files: vec![PistonFile {
                name: "solution.py".into(),
                content: code.to_string(),
            }],
            stdin: stdin.to_string(),
        };

        let resp: PistonResponse = self.client
            .post(&url)
            .json(&body)
            .timeout(std::time::Duration::from_secs(15))
            .send().await
            .context("piston request failed")?
            .json().await
            .context("piston response parse failed")?;

        Ok(resp.run)
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
