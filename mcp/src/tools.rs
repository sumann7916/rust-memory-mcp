use std::time::{SystemTime, UNIX_EPOCH};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::providers::{EmbedderProvider, LLMProvider};
use crate::qdrant::{MemoryPayload, QdrantStore};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SaveMemoryParams {
    pub content: String,
    pub user_id: String,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub module: Option<String>,
    #[serde(default)]
    pub lang: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchMemoryParams {
    pub query: String,
    pub user_id: String,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub module: Option<String>,
    #[serde(default)]
    pub lang: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    5
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetAllMemoriesParams {
    pub user_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DeleteMemoryParams {
    pub memory_id: String,
    pub user_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryItem {
    pub memory_id: String,
    pub content: String,
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    pub timestamp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f32>,
}

impl From<MemoryPayload> for MemoryItem {
    fn from(p: MemoryPayload) -> Self {
        Self {
            memory_id: p.memory_id,
            content: p.content,
            category: p.category,
            repo: p.repo,
            module: p.module,
            lang: p.lang,
            timestamp: p.timestamp,
            score: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveResult {
    pub success: bool,
    pub message: String,
    pub memory: MemoryItem,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub results: Vec<MemoryItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AllMemoriesResult {
    pub memories: Vec<MemoryItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteResult {
    pub success: bool,
    pub message: String,
}

const EXTRACT_FACT_PROMPT: &str = r#"Extract the single most important fact from the following and return only that fact as one concise sentence. No explanation, no preamble.

Input: "#;

const CLASSIFY_CATEGORY_PROMPT: &str = r#"Classify the following memory into exactly one category. Reply with only the category word, nothing else.
Categories: preference, business_logic, repo, general

Rules:
- preference: coding habits, style choices, personal patterns that apply universally
- business_logic: domain rules, product decisions, workflow rules
- repo: knowledge specific to one codebase (architecture, quirks, conventions)
- general: everything else

Memory: "#;

const GLOBAL_CHECK_PROMPT: &str = r#"Does the following memory describe a universal habit or preference that applies regardless of which codebase the user is in? Reply only 'global' or 'repo-specific'.

Memory: "#;

pub async fn save_memory(
    params: SaveMemoryParams,
    llm: &dyn LLMProvider,
    embedder: &dyn EmbedderProvider,
    store: &QdrantStore,
) -> anyhow::Result<SaveResult> {
    let fact_prompt = format!("{}{}", EXTRACT_FACT_PROMPT, params.content);
    let fact = llm.complete(&fact_prompt).await?;

    let category_prompt = format!("{}{}", CLASSIFY_CATEGORY_PROMPT, fact);
    let category_raw = llm.complete(&category_prompt).await?;
    let category = normalize_category(&category_raw);

    let global_prompt = format!("{}{}", GLOBAL_CHECK_PROMPT, fact);
    let global_response = llm.complete(&global_prompt).await?;
    let is_global = global_response.to_lowercase().contains("global")
        && !global_response.to_lowercase().contains("repo-specific");

    let repo = if is_global { None } else { params.repo };

    let embedding = embedder.embed(&fact).await?;

    let memory_id = Uuid::new_v4().to_string();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let payload = MemoryPayload {
        memory_id: memory_id.clone(),
        content: fact.clone(),
        user_id: params.user_id.clone(),
        category: category.clone(),
        repo: repo.clone(),
        module: params.module.clone(),
        lang: params.lang.clone(),
        timestamp,
    };

    store.upsert(&memory_id, embedding, payload).await?;

    let memory_item = MemoryItem {
        memory_id,
        content: fact,
        category,
        repo,
        module: params.module,
        lang: params.lang,
        timestamp,
        score: None,
    };

    Ok(SaveResult {
        success: true,
        message: format!("Memory saved for user {}", params.user_id),
        memory: memory_item,
    })
}

fn normalize_category(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if lower.contains("preference") {
        "preference".to_string()
    } else if lower.contains("business_logic") || lower.contains("business logic") {
        "business_logic".to_string()
    } else if lower.contains("repo") {
        "repo".to_string()
    } else {
        "general".to_string()
    }
}

pub async fn search_memory(
    params: SearchMemoryParams,
    embedder: &dyn EmbedderProvider,
    store: &QdrantStore,
) -> anyhow::Result<SearchResult> {
    let embedding = embedder.embed(&params.query).await?;

    let results = store
        .search(
            embedding,
            &params.user_id,
            params.repo.as_deref(),
            params.module.as_deref(),
            params.lang.as_deref(),
            params.limit as u64,
        )
        .await?;

    let items: Vec<MemoryItem> = results
        .into_iter()
        .map(|(payload, score)| {
            let mut item = MemoryItem::from(payload);
            item.score = Some(score);
            item
        })
        .collect();

    Ok(SearchResult { results: items })
}

pub async fn get_all_memories(
    params: GetAllMemoriesParams,
    store: &QdrantStore,
) -> anyhow::Result<AllMemoriesResult> {
    let payloads = store.scroll(&params.user_id).await?;
    let memories: Vec<MemoryItem> = payloads.into_iter().map(MemoryItem::from).collect();
    Ok(AllMemoriesResult { memories })
}

pub async fn delete_memory(
    params: DeleteMemoryParams,
    store: &QdrantStore,
) -> anyhow::Result<DeleteResult> {
    store.delete(&params.memory_id, &params.user_id).await?;
    Ok(DeleteResult {
        success: true,
        message: format!("Memory {} deleted", params.memory_id),
    })
}
