use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LLMProviderType {
    Ollama,
    OpenAI,
    Gemini,
    Anthropic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbedderProviderType {
    Ollama,
    OpenAI,
    Gemini,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub llm_provider: LLMProviderType,
    pub llm_model: Option<String>,
    pub embedder_provider: EmbedderProviderType,
    pub embedder_model: Option<String>,
    pub ollama_base_url: String,
    pub openai_api_key: Option<String>,
    pub gemini_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub groq_api_key: Option<String>,
    pub qdrant_host: String,
    pub qdrant_port: u16,
    pub qdrant_collection: String,
    pub memory_score_threshold: f32,
    pub memory_dedup_threshold: f32,
    pub memory_max_results: usize,
    pub scope_boost_lang: f32,
    pub scope_boost_feature: f32,
    pub scope_boost_repo: f32,
    pub scope_boost_module: f32,
    pub topic_boost_max: f32,
    pub agent_provider: Option<LLMProviderType>,
    pub agent_model: Option<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let llm_provider = match env::var("LLM_PROVIDER")
            .unwrap_or_else(|_| "ollama".to_string())
            .to_lowercase()
            .as_str()
        {
            "ollama" => LLMProviderType::Ollama,
            "openai" => LLMProviderType::OpenAI,
            "gemini" => LLMProviderType::Gemini,
            "anthropic" => LLMProviderType::Anthropic,
            other => anyhow::bail!("Unknown LLM provider: {}", other),
        };

        let embedder_provider = match env::var("EMBEDDER_PROVIDER")
            .unwrap_or_else(|_| "ollama".to_string())
            .to_lowercase()
            .as_str()
        {
            "ollama" => EmbedderProviderType::Ollama,
            "openai" => EmbedderProviderType::OpenAI,
            "gemini" => EmbedderProviderType::Gemini,
            other => anyhow::bail!("Unknown embedder provider: {}", other),
        };

        let openai_api_key = env::var("OPENAI_API_KEY").ok();
        let gemini_api_key = env::var("GEMINI_API_KEY").ok();
        let anthropic_api_key = env::var("ANTHROPIC_API_KEY").ok();
        let groq_api_key = env::var("GROQ_API_KEY").ok();

        let agent_provider = match env::var("AGENT_PROVIDER")
            .ok()
            .map(|s| s.to_lowercase())
            .as_deref()
        {
            Some("ollama") => Some(LLMProviderType::Ollama),
            Some("openai") => Some(LLMProviderType::OpenAI),
            Some("gemini") => Some(LLMProviderType::Gemini),
            Some("anthropic") => Some(LLMProviderType::Anthropic),
            _ => None,
        };
        let agent_model = env::var("AGENT_MODEL").ok();

        if llm_provider == LLMProviderType::OpenAI && openai_api_key.is_none() {
            anyhow::bail!("OPENAI_API_KEY required for OpenAI LLM provider");
        }
        if llm_provider == LLMProviderType::Gemini && gemini_api_key.is_none() {
            anyhow::bail!("GEMINI_API_KEY required for Gemini LLM provider");
        }
        if llm_provider == LLMProviderType::Anthropic && anthropic_api_key.is_none() {
            anyhow::bail!("ANTHROPIC_API_KEY required for Anthropic LLM provider");
        }
        if embedder_provider == EmbedderProviderType::OpenAI && openai_api_key.is_none() {
            anyhow::bail!("OPENAI_API_KEY required for OpenAI embedder provider");
        }
        if embedder_provider == EmbedderProviderType::Gemini && gemini_api_key.is_none() {
            anyhow::bail!("GEMINI_API_KEY required for Gemini embedder provider");
        }
        if agent_provider == Some(LLMProviderType::Gemini) && gemini_api_key.is_none() {
            anyhow::bail!("GEMINI_API_KEY required when AGENT_PROVIDER=gemini");
        }

        Ok(Config {
            llm_provider,
            llm_model: env::var("LLM_MODEL").ok(),
            embedder_provider,
            embedder_model: env::var("EMBEDDER_MODEL").ok(),
            ollama_base_url: env::var("OLLAMA_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:11434".to_string()),
            openai_api_key,
            gemini_api_key,
            anthropic_api_key,
            groq_api_key,
            agent_provider,
            agent_model,
            qdrant_host: env::var("QDRANT_HOST").unwrap_or_else(|_| "localhost".to_string()),
            qdrant_port: env::var("QDRANT_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(6334),
            qdrant_collection: env::var("QDRANT_COLLECTION")
                .unwrap_or_else(|_| "coding_memories".to_string()),
            memory_score_threshold: env::var("MEMORY_SCORE_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.2),
            memory_dedup_threshold: env::var("MEMORY_DEDUP_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.90),
            memory_max_results: env::var("MEMORY_MAX_RESULTS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            scope_boost_lang: env::var("SCOPE_BOOST_LANG")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1.3),
            scope_boost_feature: env::var("SCOPE_BOOST_FEATURE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1.4),
            scope_boost_repo: env::var("SCOPE_BOOST_REPO")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1.6),
            scope_boost_module: env::var("SCOPE_BOOST_MODULE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2.0),
            topic_boost_max: env::var("TOPIC_BOOST_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.3),
        })
    }

    pub fn default_llm_model(&self) -> &'static str {
        match self.llm_provider {
            LLMProviderType::Ollama => "qwen2.5-coder:7b",
            LLMProviderType::OpenAI => "gpt-4o-mini",
            LLMProviderType::Gemini => "gemini-2.5-flash",
            LLMProviderType::Anthropic => "claude-haiku-4-5",
        }
    }

    pub fn default_embedder_model(&self) -> &'static str {
        match self.embedder_provider {
            EmbedderProviderType::Ollama => "nomic-embed-text",
            EmbedderProviderType::OpenAI => "text-embedding-3-small",
            EmbedderProviderType::Gemini => "models/text-embedding-004",
        }
    }

    pub fn llm_model(&self) -> &str {
        self.llm_model
            .as_deref()
            .unwrap_or_else(|| self.default_llm_model())
    }

    pub fn embedder_model(&self) -> &str {
        self.embedder_model
            .as_deref()
            .unwrap_or_else(|| self.default_embedder_model())
    }

    pub fn vector_size(&self) -> usize {
        match self.embedder_provider {
            EmbedderProviderType::Ollama => 768,
            EmbedderProviderType::OpenAI => 1536,
            EmbedderProviderType::Gemini => 768,
        }
    }
}
