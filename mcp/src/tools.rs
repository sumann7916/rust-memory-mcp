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
    pub file_path: Option<String>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modules: Vec<String>,
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
            modules: p.modules,
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

const MODULE_SCOPE_PROMPT: &str = r#"Analyze this memory and the file path context.
Does this memory apply to:
- 'single': Only the current module/directory
- 'multiple': Multiple specific modules (list them based on what the memory describes)
- 'repo': The entire repository (architecture decisions, repo-wide conventions, etc.)

File path: {file_path}
Memory: {content}

Reply with ONLY valid JSON, no other text:
{"scope": "single", "modules": []} for single module (modules will be derived from file path)
{"scope": "multiple", "modules": ["module1", "module2"]} for multiple modules
{"scope": "repo", "modules": []} for repo-wide"#;

#[derive(Debug, Deserialize)]
struct ModuleScopeResponse {
    scope: String,
    #[serde(default)]
    modules: Vec<String>,
}

/// Extract module (directory) from a file path.
/// e.g. "highway/index.js" -> "highway"
///      "./src/auth/login.ts" -> "src/auth"
///      "service.js" -> None (root level)
fn extract_module_from_path(file_path: Option<&str>) -> Option<String> {
    let s = file_path?.trim();
    if s.is_empty() {
        return None;
    }
    let s = s.replace('\\', "/");
    let s = s.strip_prefix("./").unwrap_or(&s);
    let s = s.strip_prefix('/').unwrap_or(s);
    if s.is_empty() {
        return None;
    }
    if let Some((dir, _)) = s.rsplit_once('/') {
        if dir.is_empty() {
            return None;
        }
        return Some(dir.to_string());
    }
    None
}

/// Normalize a module name for consistent storage/search.
fn normalize_module_name(module: &str) -> String {
    let s = module.trim();
    let s = s.replace('\\', "/");
    let s = s.strip_prefix("./").unwrap_or(&s);
    let s = s.strip_prefix('/').unwrap_or(s);
    let s = s.strip_suffix('/').unwrap_or(s);
    s.to_string()
}

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

    let modules = if is_global {
        vec![]
    } else {
        determine_modules(&fact, params.file_path.as_deref(), llm).await?
    };

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
        modules: modules.clone(),
        lang: params.lang.clone(),
        timestamp,
    };

    store.upsert(&memory_id, embedding, payload).await?;

    let memory_item = MemoryItem {
        memory_id,
        content: fact,
        category,
        repo,
        modules,
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

async fn determine_modules(
    content: &str,
    file_path: Option<&str>,
    llm: &dyn LLMProvider,
) -> anyhow::Result<Vec<String>> {
    let file_path_str = file_path.unwrap_or("(not provided)");
    let prompt = MODULE_SCOPE_PROMPT
        .replace("{file_path}", file_path_str)
        .replace("{content}", content);

    let response = llm.complete(&prompt).await?;

    let scope_response: ModuleScopeResponse = match serde_json::from_str(&response) {
        Ok(r) => r,
        Err(_) => {
            if let Some(module) = extract_module_from_path(file_path) {
                return Ok(vec![module]);
            }
            return Ok(vec![]);
        }
    };

    match scope_response.scope.as_str() {
        "single" => {
            if let Some(module) = extract_module_from_path(file_path) {
                Ok(vec![module])
            } else {
                Ok(vec![])
            }
        }
        "multiple" => {
            let normalized: Vec<String> = scope_response
                .modules
                .iter()
                .map(|m| normalize_module_name(m))
                .filter(|m| !m.is_empty())
                .collect();
            if normalized.is_empty() {
                if let Some(module) = extract_module_from_path(file_path) {
                    return Ok(vec![module]);
                }
            }
            Ok(normalized)
        }
        "repo" => Ok(vec![]),
        _ => {
            if let Some(module) = extract_module_from_path(file_path) {
                Ok(vec![module])
            } else {
                Ok(vec![])
            }
        }
    }
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
    let module = extract_module_from_path(params.module.as_deref());

    let results = store
        .search(
            embedding,
            &params.user_id,
            params.repo.as_deref(),
            module.as_deref(),
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
