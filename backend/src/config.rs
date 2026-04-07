#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub piston_url: String,
    pub openai_api_key: String,
    pub anthropic_api_key: String,
    pub google_api_key: String,
    pub xai_api_key: String,
    pub fireworks_api_key: String,
    pub deepseek_api_key: String,
    pub qwen_api_key: String,
    pub moonshot_api_key: String,
    pub doubao_api_key: String,
    pub hunyuan_api_key: String,
    pub mistral_api_key: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://localhost/howfastwould".into()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "3001".into())
                .parse()?,
            piston_url: std::env::var("PISTON_URL")
                .unwrap_or_else(|_| "https://emkc.org/api/v2/piston".into()),
            openai_api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            google_api_key: std::env::var("GOOGLE_API_KEY").unwrap_or_default(),
            xai_api_key: std::env::var("XAI_API_KEY").unwrap_or_default(),
            fireworks_api_key: std::env::var("FIREWORKS_API_KEY").unwrap_or_default(),
            deepseek_api_key: std::env::var("DEEPSEEK_API_KEY").unwrap_or_default(),
            qwen_api_key: std::env::var("QWEN_API_KEY").unwrap_or_default(),
            moonshot_api_key: std::env::var("MOONSHOT_API_KEY").unwrap_or_default(),
            doubao_api_key: std::env::var("DOUBAO_API_KEY").unwrap_or_default(),
            hunyuan_api_key: std::env::var("HUNYUAN_API_KEY").unwrap_or_default(),
            mistral_api_key: std::env::var("MISTRAL_API_KEY").unwrap_or_default(),
        })
    }
}
