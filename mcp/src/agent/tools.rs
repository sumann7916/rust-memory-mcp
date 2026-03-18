use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

use crate::config::Config;
use crate::providers::EmbedderProvider;
use crate::qdrant::QdrantStore;
use crate::tools::{
    self, GetAllMemoriesParams, GetMemoryIndexParams, SaveMemoryParams, SearchMemoryParams,
};

use super::traits::Tool;
use crate::agent::agents::memory_chat::SearchConfig;

// ============================================================================
// SearchMemoryTool
// ============================================================================

pub struct SearchMemoryTool {
    embedder: Arc<Box<dyn EmbedderProvider>>,
    store: Arc<QdrantStore>,
    config: Arc<Config>,
    search_config: Arc<Mutex<SearchConfig>>,
}

impl SearchMemoryTool {
    pub fn new(
        embedder: Arc<Box<dyn EmbedderProvider>>,
        store: Arc<QdrantStore>,
        config: Arc<Config>,
        search_config: Arc<Mutex<SearchConfig>>,
    ) -> Self {
        Self {
            embedder,
            store,
            config,
            search_config,
        }
    }
}

#[async_trait]
impl Tool for SearchMemoryTool {
    fn name(&self) -> &str {
        "search_memory"
    }

    fn description(&self) -> &str {
        "Search memories by semantic similarity. Returns memories ranked by relevance with boost scoring based on scope, topic overlap, and confidence."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "user_id": {
                    "type": "string",
                    "description": "User ID to search memories for"
                },
                "repo": {
                    "type": "string",
                    "description": "Repository name for context boost"
                },
                "module": {
                    "type": "string",
                    "description": "Module/file path for context boost"
                },
                "lang": {
                    "type": "string",
                    "description": "Programming language for context boost"
                },
                "feature": {
                    "type": "string",
                    "description": "Feature name for context boost"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results (default: 5)"
                },
                "min_score": {
                    "type": "number",
                    "description": "Minimum score threshold"
                }
            },
            "required": ["query", "user_id"]
        })
    }

    async fn call(&self, args: Value) -> Result<Value> {
        let mut params: SearchMemoryParams = serde_json::from_value(args)?;
        if params.min_score.is_none() {
            let config = self.search_config.lock().unwrap();
            params.min_score = config.min_score;
        }
        if params.limit == 5 {
            let config = self.search_config.lock().unwrap();
            params.limit = config.default_limit;
        }
        let result = tools::search_memory(params, &**self.embedder, &self.store, &self.config).await?;
        Ok(serde_json::to_value(result)?)
    }
}

// ============================================================================
// SaveMemoryTool
// ============================================================================

pub struct SaveMemoryTool {
    embedder: Arc<Box<dyn EmbedderProvider>>,
    store: Arc<QdrantStore>,
    config: Arc<Config>,
}

impl SaveMemoryTool {
    pub fn new(
        embedder: Arc<Box<dyn EmbedderProvider>>,
        store: Arc<QdrantStore>,
        config: Arc<Config>,
    ) -> Self {
        Self {
            embedder,
            store,
            config,
        }
    }
}

#[async_trait]
impl Tool for SaveMemoryTool {
    fn name(&self) -> &str {
        "save_memory"
    }

    fn description(&self) -> &str {
        "Save a new memory. Required: content, user_id, scope (one of: global/lang/feature/repo/module). Optional: topics, repo, lang, module, feature. Scope determines memory reach: module (most specific, needs repo+module), repo (needs repo), feature (needs feature), lang (needs lang), global (no extra fields). Automatically handles deduplication and contradiction detection."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "The memory content to save"
                },
                "user_id": {
                    "type": "string",
                    "description": "User ID to save memory for"
                },
                "topics": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Topics/tags for the memory (optional, defaults to empty array)"
                },
                "scope": {
                    "type": "string",
                    "enum": ["global", "lang", "feature", "repo", "module"],
                    "description": "REQUIRED. Scope of the memory: 'global' (universal), 'lang' (language-specific, requires lang param), 'feature' (product feature, requires feature param), 'repo' (repository-specific, requires repo param), 'module' (module/directory, requires repo+module params)"
                },
                "repo": {
                    "type": "string",
                    "description": "Repository name (required for 'repo' and 'module' scopes)"
                },
                "lang": {
                    "type": "string",
                    "description": "Programming language (required for 'lang' scope)"
                },
                "module": {
                    "type": "string",
                    "description": "Module/file path like 'src/auth' (required for 'module' scope)"
                },
                "feature": {
                    "type": "string",
                    "description": "Feature name (required for 'feature' scope)"
                }
            },
            "required": ["content", "user_id", "scope"]
        })
    }

    async fn call(&self, args: Value) -> Result<Value> {
        let params: SaveMemoryParams = serde_json::from_value(args)?;
        let result = tools::save_memory(params, &**self.embedder, &self.store, &self.config).await?;
        Ok(serde_json::to_value(result)?)
    }
}

// ============================================================================
// GetMemoryIndexTool
// ============================================================================

pub struct GetMemoryIndexTool {
    store: Arc<QdrantStore>,
}

impl GetMemoryIndexTool {
    pub fn new(store: Arc<QdrantStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for GetMemoryIndexTool {
    fn name(&self) -> &str {
        "get_memory_index"
    }

    fn description(&self) -> &str {
        "Get an overview of all memories: topics, languages, repos, scope distribution, and recent memories."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "user_id": {
                    "type": "string",
                    "description": "User ID to get index for"
                }
            },
            "required": ["user_id"]
        })
    }

    async fn call(&self, args: Value) -> Result<Value> {
        let params: GetMemoryIndexParams = serde_json::from_value(args)?;
        let result = tools::get_memory_index(params, &self.store).await?;
        Ok(serde_json::to_value(result)?)
    }
}

// ============================================================================
// GetAllMemoriesTool
// ============================================================================

pub struct GetAllMemoriesTool {
    store: Arc<QdrantStore>,
}

impl GetAllMemoriesTool {
    pub fn new(store: Arc<QdrantStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for GetAllMemoriesTool {
    fn name(&self) -> &str {
        "get_all_memories"
    }

    fn description(&self) -> &str {
        "Get all memories for a user, including superseded ones."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "user_id": {
                    "type": "string",
                    "description": "User ID to get memories for"
                }
            },
            "required": ["user_id"]
        })
    }

    async fn call(&self, args: Value) -> Result<Value> {
        let params: GetAllMemoriesParams = serde_json::from_value(args)?;
        let result = tools::get_all_memories(params, &self.store).await?;
        Ok(serde_json::to_value(result)?)
    }
}

// ============================================================================
// ConfigureSearchTool
// ============================================================================

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ConfigureSearchParams {
    #[serde(default)]
    min_score: Option<f32>,
    #[serde(default)]
    limit: Option<usize>,
}

pub struct ConfigureSearchTool {
    config: Arc<Mutex<SearchConfig>>,
}

impl ConfigureSearchTool {
    pub fn new(config: Arc<Mutex<SearchConfig>>) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Tool for ConfigureSearchTool {
    fn name(&self) -> &str {
        "configure_search"
    }

    fn description(&self) -> &str {
        "Configure search parameters. Settings persist across all future searches in this session. Use when user requests to change min_score threshold or result limit."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "min_score": {
                    "type": "number",
                    "description": "Minimum similarity score threshold (0.0-1.0). Lower values return more results. Default: 0.2"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results to return (1-10). Default: 5"
                }
            }
        })
    }

    async fn call(&self, args: Value) -> Result<Value> {
        let params: ConfigureSearchParams = serde_json::from_value(args)?;
        let mut config = self.config.lock().unwrap();
        
        let mut updated = Vec::new();
        
        if let Some(min_score) = params.min_score {
            config.min_score = Some(min_score.max(0.0).min(1.0));
            updated.push(format!("min_score={}", min_score));
        }
        
        if let Some(limit) = params.limit {
            config.default_limit = limit.max(1).min(10);
            updated.push(format!("limit={}", limit));
        }
        
        let message = if updated.is_empty() {
            "No parameters updated".to_string()
        } else {
            format!("Search configuration updated: {}", updated.join(", "))
        };
        
        Ok(json!({
            "success": true,
            "message": message,
            "current_config": {
                "min_score": config.min_score,
                "limit": config.default_limit
            }
        }))
    }
}
