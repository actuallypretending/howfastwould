use crate::models::RaceResultWithModel;
use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};

pub struct RoastGenerator {
    client: Client,
    api_key: String,
}

impl RoastGenerator {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
        }
    }

    pub async fn generate(
        &self,
        winner: &RaceResultWithModel,
        loser: &RaceResultWithModel,
        problem_title: &str,
    ) -> String {
        match self.call_api(winner, loser, problem_title).await {
            Ok(line) => line,
            Err(_) => self.fallback_roast(winner, loser),
        }
    }

    async fn call_api(
        &self,
        winner: &RaceResultWithModel,
        loser: &RaceResultWithModel,
        problem_title: &str,
    ) -> Result<String> {
        let winner_time = format_time(winner.time_ms);
        let loser_time = format_time(loser.time_ms);

        let prompt = format!(
            r#"Write ONE short roast (max 12 words, no punctuation at end) in the style of a savage sports commentator.
{} solved {} in {} while {} took {}.
Make it meme-worthy. Reference both competitors. No hashtags."#,
            winner.display_name, problem_title, winner_time,
            loser.display_name, loser_time
        );

        let body = json!({
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": 60,
            "messages": [{ "role": "user", "content": prompt }]
        });

        let resp: Value = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send().await?
            .json().await?;

        let line = resp["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();

        if line.is_empty() {
            anyhow::bail!("empty response");
        }
        Ok(line)
    }

    fn fallback_roast(&self, winner: &RaceResultWithModel, loser: &RaceResultWithModel) -> String {
        let templates = [
            format!("{} left {} in the dust", winner.display_name, loser.display_name),
            format!("{} finished before {} even read the problem", winner.display_name, loser.display_name),
            format!("{} said hold my API key", winner.display_name),
            format!("{} is still loading", loser.display_name),
        ];
        let idx = (winner.time_ms.unwrap_or(0) % templates.len() as i64).unsigned_abs() as usize;
        templates[idx].clone()
    }
}

pub fn format_time(ms: Option<i64>) -> String {
    match ms {
        None => "DNF".into(),
        Some(ms) if ms < 1000 => format!("{}ms", ms),
        Some(ms) if ms < 60_000 => format!("{:.1}s", ms as f64 / 1000.0),
        Some(ms) => {
            let mins = ms / 60_000;
            let secs = (ms % 60_000) / 1000;
            format!("{}m {}s", mins, secs)
        }
    }
}
