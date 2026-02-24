mod config;
mod providers;
mod qdrant;
mod tools;

use rmcp::{
    handler::server::{
        router::prompt::PromptRouter, tool::ToolRouter, wrapper::Parameters,
    },
    model::*,
    prompt, prompt_handler, prompt_router, tool, tool_handler, tool_router,
    service::RequestContext,
    transport::stdio,
    ErrorData as McpError, RoleServer, ServerHandler, ServiceExt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use config::Config;
use providers::{create_embedder, create_llm, EmbedderProvider, LLMProvider};
use qdrant::QdrantStore;
use tools::{DeleteMemoryParams, GetAllMemoriesParams, SaveMemoryParams, SearchMemoryParams};

pub struct MemoryServer {
    tool_router: ToolRouter<Self>,
    prompt_router: PromptRouter<Self>,
    llm: Arc<Box<dyn LLMProvider>>,
    embedder: Arc<Box<dyn EmbedderProvider>>,
    store: Arc<QdrantStore>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MemoryInstructionsParams {}

const MEMORY_INSTRUCTIONS: &str = r#"You have access to memory tools. Follow these rules:

BEFORE every response:
- Call search_memory with the user's question as query
- Pass current repo and lang if you know them from context
- This automatically returns both your global memories (habits, preferences) and repo-specific memories
- Use returned memories to inform your response

WHEN saving memories — call save_memory when:
- User corrects you or shows a preference
- You solve a non-obvious problem
- User states a rule or convention
- Something would be useful to remember next time

GLOBAL vs REPO-SPECIFIC — you do not need to decide this, the server handles it automatically based on content."#;

#[tool_router]
impl MemoryServer {
    fn new(
        llm: Box<dyn LLMProvider>,
        embedder: Box<dyn EmbedderProvider>,
        store: QdrantStore,
    ) -> Self {
        Self {
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
            llm: Arc::new(llm),
            embedder: Arc::new(embedder),
            store: Arc::new(store),
        }
    }

    #[tool(description = "Save a memory. The server extracts the key fact, classifies it, and determines scope (single module, multiple modules, or repo-wide). Parameters: content (required), user_id (required), repo (optional - e.g. portpro-backend), file_path (optional - current file path from editor, e.g. src/auth/login.ts; server extracts module from directory and LLM decides if memory applies to single/multiple/all modules), lang (optional). Call proactively when you learn something useful or the user shows a preference.")]
    async fn save_memory(
        &self,
        params: Parameters<SaveMemoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let result = tools::save_memory(
            params.0,
            self.llm.as_ref().as_ref(),
            self.embedder.as_ref().as_ref(),
            self.store.as_ref(),
        )
        .await
        .map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: e.to_string().into(),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap(),
        )]))
    }

    #[tool(description = "Search memories by semantic similarity. Returns global + repo-wide + module-specific memories. Parameters: query (required), user_id (required), repo (optional - filters to global OR this repo), module (optional - current file path from editor, e.g. src/auth/login.ts; extracts directory and returns memories with empty modules OR containing this module), lang (optional), limit (optional, default 5). Call before generating code or plans.")]
    async fn search_memory(
        &self,
        params: Parameters<SearchMemoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let result = tools::search_memory(
            params.0,
            self.embedder.as_ref().as_ref(),
            self.store.as_ref(),
        )
        .await
        .map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: e.to_string().into(),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap(),
        )]))
    }

    #[tool(description = "Get all memories for a user. Returns complete list with all metadata. Only call when user explicitly asks to see all memories.")]
    async fn get_all_memories(
        &self,
        params: Parameters<GetAllMemoriesParams>,
    ) -> Result<CallToolResult, McpError> {
        let result = tools::get_all_memories(params.0, self.store.as_ref())
            .await
            .map_err(|e| McpError {
                code: ErrorCode::INTERNAL_ERROR,
                message: e.to_string().into(),
                data: None,
            })?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap(),
        )]))
    }

    #[tool(description = "Delete a specific memory by ID. Requires memory_id and user_id. Only call when user explicitly asks to delete a memory.")]
    async fn delete_memory(
        &self,
        params: Parameters<DeleteMemoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let result = tools::delete_memory(params.0, self.store.as_ref())
            .await
            .map_err(|e| McpError {
                code: ErrorCode::INTERNAL_ERROR,
                message: e.to_string().into(),
                data: None,
            })?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap(),
        )]))
    }
}

#[prompt_router]
impl MemoryServer {
    #[prompt(
        name = "memory-instructions",
        description = "Instructions for using memory tools effectively"
    )]
    async fn memory_instructions(
        &self,
        _params: Parameters<MemoryInstructionsParams>,
    ) -> Result<GetPromptResult, McpError> {
        Ok(GetPromptResult {
            description: Some("Instructions for using memory tools".into()),
            messages: vec![PromptMessage::new_text(
                PromptMessageRole::User,
                MEMORY_INSTRUCTIONS,
            )],
        })
    }
}

#[tool_handler]
#[prompt_handler]
impl ServerHandler for MemoryServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Memory server for storing and retrieving coding lessons, facts, and context. \
                 Use search_memory before generating code to check for relevant past lessons. \
                 Use save_memory proactively when solutions work, bugs are fixed, or patterns are learned."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .build(),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;

    let llm = create_llm(&config);
    let embedder = create_embedder(&config);

    let store = QdrantStore::new(
        &config.qdrant_host,
        config.qdrant_port,
        config.qdrant_collection.clone(),
        config.vector_size() as u64,
    )
    .await?;

    let service = MemoryServer::new(llm, embedder, store)
        .serve(stdio())
        .await
        .inspect_err(|e| eprintln!("Error starting server: {}", e))?;

    service.waiting().await?;
    Ok(())
}
