use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{EmbedderProvider, LLMProvider};

pub struct GeminiProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl GeminiProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Serialize)]
struct EmbeddingRequest<'a> {
    content: EmbeddingContent<'a>,
}

#[derive(Serialize)]
struct EmbeddingContent<'a> {
    parts: Vec<EmbeddingPart<'a>>,
}

#[derive(Serialize)]
struct EmbeddingPart<'a> {
    text: &'a str,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    embedding: EmbeddingValues,
}

#[derive(Deserialize)]
struct EmbeddingValues {
    values: Vec<f32>,
}

#[derive(Serialize)]
struct GenerateRequest<'a> {
    contents: Vec<GenerateContent<'a>>,
}

#[derive(Serialize)]
struct GenerateContent<'a> {
    parts: Vec<GeneratePart<'a>>,
}

#[derive(Serialize)]
struct GeneratePart<'a> {
    text: &'a str,
}

#[derive(Deserialize)]
struct GenerateResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: CandidateContent,
}

#[derive(Deserialize)]
struct CandidateContent {
    parts: Vec<CandidatePart>,
}

#[derive(Deserialize)]
struct CandidatePart {
    text: String,
}

#[async_trait]
impl EmbedderProvider for GeminiProvider {
    async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/{}:embedContent?key={}",
            self.model, self.api_key
        );

        let request = EmbeddingRequest {
            content: EmbeddingContent {
                parts: vec![EmbeddingPart { text }],
            },
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?
            .error_for_status()?
            .json::<EmbeddingResponse>()
            .await?;

        Ok(response.embedding.values)
    }
}

#[async_trait]
impl LLMProvider for GeminiProvider {
    async fn complete(&self, prompt: &str) -> anyhow::Result<String> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let request = GenerateRequest {
            contents: vec![GenerateContent {
                parts: vec![GeneratePart { text: prompt }],
            }],
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?
            .error_for_status()?
            .json::<GenerateResponse>()
            .await?;

        response
            .candidates
            .into_iter()
            .next()
            .and_then(|c| c.content.parts.into_iter().next())
            .map(|p| p.text.trim().to_string())
            .ok_or_else(|| anyhow::anyhow!("No completion returned"))
    }
}
