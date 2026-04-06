use crate::models::{Problem, TestCase};
use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::{json, Value};
use uuid::Uuid;
use chrono::Utc;

const LC_GRAPHQL: &str = "https://leetcode.com/graphql";

pub struct LeetcodeClient {
    client: Client,
}

impl LeetcodeClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0")
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap(),
        }
    }

    pub async fn fetch_random_problem(&self) -> Result<Problem> {
        let slug = self.fetch_random_slug().await?;
        self.fetch_problem_by_slug(&slug).await
    }

    async fn fetch_random_slug(&self) -> Result<String> {
        let query = json!({
            "query": r#"
                query randomQuestion($categorySlug: String, $filters: QuestionListFilterInput) {
                    randomQuestion(categorySlug: $categorySlug, filters: $filters) {
                        titleSlug
                    }
                }
            "#,
            "variables": { "categorySlug": "", "filters": {} }
        });

        let resp: Value = self.client
            .post(LC_GRAPHQL)
            .json(&query)
            .send().await?
            .json().await?;

        let slug = resp["data"]["randomQuestion"]["titleSlug"]
            .as_str()
            .context("missing titleSlug")?
            .to_string();
        Ok(slug)
    }

    async fn fetch_problem_by_slug(&self, slug: &str) -> Result<Problem> {
        let query = json!({
            "query": r#"
                query questionData($titleSlug: String!) {
                    question(titleSlug: $titleSlug) {
                        questionFrontendId
                        title
                        difficulty
                        content
                        codeSnippets { langSlug code }
                        exampleTestcaseList
                        metaData
                    }
                }
            "#,
            "variables": { "titleSlug": slug }
        });

        let resp: Value = self.client
            .post(LC_GRAPHQL)
            .json(&query)
            .send().await?
            .json().await?;

        let q = &resp["data"]["question"];

        let lc_id: i64 = q["questionFrontendId"]
            .as_str().context("missing id")?
            .parse()?;

        let title = q["title"].as_str().context("missing title")?.to_string();
        let difficulty = q["difficulty"].as_str().context("missing difficulty")?.to_string();
        let description = q["content"].as_str().unwrap_or("").to_string();

        let starter_code = q["codeSnippets"]
            .as_array()
            .and_then(|snips| snips.iter().find(|s| s["langSlug"] == "python3"))
            .and_then(|s| s["code"].as_str())
            .unwrap_or("class Solution:\n    pass")
            .to_string();

        let test_cases = self.parse_test_cases(&q["exampleTestcaseList"]);

        Ok(Problem {
            id: Uuid::new_v4().to_string(),
            lc_id,
            title,
            difficulty,
            description,
            starter_code,
            test_cases: serde_json::to_string(&test_cases)?,
            source: "leetcode".into(),
            cached_at: Utc::now().to_rfc3339(),
        })
    }

    fn parse_test_cases(&self, example_list: &Value) -> Vec<TestCase> {
        let inputs: Vec<String> = example_list
            .as_array()
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect())
            .unwrap_or_default();

        inputs.iter().map(|input| TestCase {
            input: input.clone(),
            expected_output: String::new(),
        }).collect()
    }
}

pub async fn cache_problem(pool: &sqlx::SqlitePool, problem: &Problem) -> Result<()> {
    sqlx::query!(
        r#"INSERT OR REPLACE INTO problems
           (id, lc_id, title, difficulty, description, starter_code, test_cases, source, cached_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        problem.id, problem.lc_id, problem.title, problem.difficulty,
        problem.description, problem.starter_code, problem.test_cases,
        problem.source, problem.cached_at
    ).execute(pool).await?;
    Ok(())
}

pub async fn get_random_cached(pool: &sqlx::SqlitePool) -> Result<Problem> {
    sqlx::query_as!(Problem,
        r#"SELECT id as "id!", lc_id as "lc_id!", title as "title!", difficulty as "difficulty!",
           description as "description!", starter_code as "starter_code!",
           test_cases as "test_cases!", source as "source!", cached_at as "cached_at!"
           FROM problems ORDER BY RANDOM() LIMIT 1"#
    ).fetch_one(pool).await.context("no cached problems")
}

pub async fn search_problems(pool: &sqlx::SqlitePool, q: &str) -> Result<Vec<Problem>> {
    let escaped = q.replace('%', "\\%").replace('_', "\\_");
    let pattern = format!("%{}%", escaped);
    sqlx::query_as!(Problem,
        r#"SELECT id as "id!", lc_id as "lc_id!", title as "title!", difficulty as "difficulty!",
           description as "description!", starter_code as "starter_code!",
           test_cases as "test_cases!", source as "source!", cached_at as "cached_at!"
           FROM problems WHERE title LIKE ? ESCAPE '\\' OR CAST(lc_id AS TEXT) = ? OR difficulty = ?
           LIMIT 20"#,
        pattern, q, q
    ).fetch_all(pool).await.context("search failed")
}
