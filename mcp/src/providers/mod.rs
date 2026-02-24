pub mod anthropic;
pub mod gemini;
pub mod ollama;
pub mod openai;

use async_trait::async_trait;

use crate::config::{Config, EmbedderProviderType, LLMProviderType};

#[async_trait]
pub trait EmbedderProvider: Send + Sync {
    async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>>;
}

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn complete(&self, prompt: &str) -> anyhow::Result<String>;
}

pub fn create_embedder(config: &Config) -> Box<dyn EmbedderProvider> {
    match config.embedder_provider {
        EmbedderProviderType::Ollama => Box::new(ollama::OllamaProvider::new(
            config.ollama_base_url.clone(),
            config.embedder_model().to_string(),
        )),
        EmbedderProviderType::OpenAI => Box::new(openai::OpenAIProvider::new(
            config.openai_api_key.clone().unwrap(),
            config.embedder_model().to_string(),
        )),
        EmbedderProviderType::Gemini => Box::new(gemini::GeminiProvider::new(
            config.gemini_api_key.clone().unwrap(),
            config.embedder_model().to_string(),
        )),
    }
}

pub fn create_llm(config: &Config) -> Box<dyn LLMProvider> {
    match config.llm_provider {
        LLMProviderType::Ollama => Box::new(ollama::OllamaProvider::new(
            config.ollama_base_url.clone(),
            config.llm_model().to_string(),
        )),
        LLMProviderType::OpenAI => Box::new(openai::OpenAIProvider::new(
            config.openai_api_key.clone().unwrap(),
            config.llm_model().to_string(),
        )),
        LLMProviderType::Gemini => Box::new(gemini::GeminiProvider::new(
            config.gemini_api_key.clone().unwrap(),
            config.llm_model().to_string(),
        )),
        LLMProviderType::Anthropic => Box::new(anthropic::AnthropicProvider::new(
            config.anthropic_api_key.clone().unwrap(),
            config.llm_model().to_string(),
        )),
    }
}
