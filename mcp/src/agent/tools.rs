use std::sync::Arc;

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

// ============================================================================
// SearchMemoryTool
// ============================================================================

pub struct SearchMemoryTool {
    embedder: Arc<Box<dyn EmbedderProvider>>,
    store: Arc<QdrantStore>,
    config: Arc<Config>,
}

impl SearchMemoryTool {
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
            params.min_score = Some(0.5);
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
        "Save a new memory. Automatically handles deduplication (reinforcement if similar exists) and contradiction detection (marks old as superseded)."
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
                    "description": "Topics/tags for the memory"
                },
                "scope": {
                    "type": "string",
                    "enum": ["global", "lang", "feature", "repo", "module"],
                    "description": "Scope of the memory"
                },
                "repo": {
                    "type": "string",
                    "description": "Repository name (required for repo/module scope)"
                },
                "lang": {
                    "type": "string",
                    "description": "Programming language (required for lang scope)"
                },
                "module": {
                    "type": "string",
                    "description": "Module/file path (required for module scope)"
                },
                "feature": {
                    "type": "string",
                    "description": "Feature name (for feature scope)"
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
