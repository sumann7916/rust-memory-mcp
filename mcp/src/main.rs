use memory_mcp::config::Config;
use memory_mcp::providers::{create_embedder, EmbedderProvider};
use memory_mcp::qdrant::QdrantStore;
use memory_mcp::tools::{
    self, CorrectMemoryParams, DeleteMemoryParams, GetAllMemoriesParams,
    GetMemoryIndexParams, SaveMemoryParams, SearchMemoryParams,
};

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


pub struct MemoryServer {
    tool_router: ToolRouter<Self>,
    prompt_router: PromptRouter<Self>,
    embedder: Arc<Box<dyn EmbedderProvider>>,
    store: Arc<QdrantStore>,
    config: Arc<Config>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MemoryInstructionsParams {}

const MEMORY_INSTRUCTIONS: &str = r#"# Memory System - LLM-First Architecture

## Core Philosophy

**LLM = Intelligence** (you do all the thinking)
**Server = Storage + Boost Ranking** (dumb storage with semantic search)

## At Session Start

Call `get_memory_index(user_id)` to get:
- Existing topics (for consistency)
- Languages and repos in use
- Scope distribution
- Recent memories

Cache this for the session. When saving new memories, prefer reusing existing topics when applicable.

## Before Calling save_memory

YOU must pre-process the memory. Ask yourself:

1. **Extract the fact**: What is the single most important fact to remember?
2. **Determine topics**: What 2-5 topics does this touch? Use conversation context.
   - Check cached topics from get_memory_index
   - Reuse existing topics when applicable (e.g., "auth" not "authentication")
   - Only create new topics if existing ones don't fit
   - Technical: auth, jwt, websockets, database, async, error-handling
   - Domain: payments, invoicing, notifications, file-upload
   - Tools: tokio, serde, docker, kubernetes
3. **Decide scope** (apply this decision tree):
   - Would I want this in EVERY project I ever work on? → `global`
   - Would I want this in every [lang] project? → `lang`
   - Would I want this whenever working on [feature/domain]? → `feature`
   - Is this specific to this one repo's architecture? → `repo`
   - Is this specific to this one file/directory? → `module`

Pass the processed data to save_memory with: content, user_id, topics, scope, repo, lang, module, feature.

## When to Save

- User shows a preference or rule
- You solve a non-obvious problem
- User corrects you → use correct_memory instead
- A pattern emerges that would help in the future

## Before Generating Code

Call search_memory with:
- query: descriptive search query for what you need
- user_id: required
- repo, module, lang, feature: current context for boost ranking

## How Search Works

Server returns memories ranked by:
- Semantic similarity (base score)
- Scope boost (global always surfaces, module-match gets biggest boost)
- Topic overlap boost
- Confidence boost

Results include score and boost values. Trust memories with high confidence more than low.

## Scope Boost Logic

- `global`: boost = 1.0 (always included)
- `lang`: boost = 1.3 if lang matches
- `feature`: boost = 1.4 if feature matches  
- `repo`: boost = 1.6 if repo matches
- `module`: boost = 2.0 if module matches
- Non-matching scope: boost = 0.5 (penalty)

Global-scoped memories surface everywhere. Repo-scoped memories can still help in other repos but with lower boost.

## Example Session Flow

Session start:
→ get_memory_index(user_id="sumankhadka")
→ Cache: topics=["auth", "jwt", "security", "payments", "async"]

User: "JWT refresh tokens should be rotated on every use"

Your thought process:
→ Fact: "JWT refresh tokens should be rotated on every use and old token invalidated to prevent replay attacks"
→ Topics: Check cached topics → reuse ["auth", "jwt", "security"] (already exist)
→ Scope: "feature" (applies to auth feature across projects)
→ Context: working in typescript, auth feature

Call: save_memory(
  content="JWT refresh tokens should be rotated on every use and old token invalidated to prevent replay attacks",
  user_id="sumankhadka",
  topics=["auth", "jwt", "security"],
  scope="feature",
  lang="typescript",
  feature="auth"
)

## Example Search Flow

User asks: "How should I handle JWT tokens in my API?"

Your thought process:
→ Query: "JWT token handling and refresh strategy for APIs"
→ Context: repo=my-api, lang=rust, module=src/auth

Call: search_memory(
  query="JWT token handling and refresh strategy for APIs",
  user_id="sumankhadka",
  repo="my-api",
  lang="rust",
  module="src/auth",
  limit=5
)

Server will:
1. Semantic search (user_id filter only)
2. Calculate boosts for each result
3. Re-rank by semantic_score * boost
4. Return top 5

## Important Notes

- Call get_memory_index at session start for topic consistency
- Reuse existing topics when applicable
- Deduplication happens automatically on save
- If code contradicts memory, trust the code
- Memories are hints to inform your approach, not facts"#;

#[tool_router]
impl MemoryServer {
    fn new(
        embedder: Box<dyn EmbedderProvider>,
        store: QdrantStore,
        config: Config,
    ) -> Self {
        Self {
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
            embedder: Arc::new(embedder),
            store: Arc::new(store),
            config: Arc::new(config),
        }
    }

    #[tool(description = "Save a memory. LLM MUST pre-process before calling: (1) Extract clean single-sentence fact, (2) Determine 2-5 topics from conversation context, (3) Decide scope using decision tree (global/lang/feature/repo/module). Server validates scope, checks for duplicates (reinforces if found), detects contradictions (supersedes if found), and stores. Parameters: content (required - pre-processed fact), user_id (required), topics (required - array of topic strings), scope (required - one of: global/lang/feature/repo/module), repo (optional), lang (optional), module (optional - directory path like 'src/auth'), feature (optional - product area like 'invoicing'). Call proactively when patterns emerge or user shows preference.")]
    async fn save_memory(
        &self,
        params: Parameters<SaveMemoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let result = tools::save_memory(
            params.0,
            self.embedder.as_ref().as_ref(),
            self.store.as_ref(),
            self.config.as_ref(),
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

    #[tool(description = "Search memories by semantic similarity with boost-based ranking. Returns memories ranked by semantic_score * boost. Boost calculated from: scope match (module=2.0x, repo=1.6x, feature=1.4x, lang=1.3x, global=1.0x), topic overlap, and confidence. Only user_id is hard-filtered; repo/lang/module/feature are boost signals. Results include score and boost values. Parameters: query (required - descriptive search query), user_id (required), repo (optional - for boost), module (optional - directory path for boost), lang (optional - for boost), feature (optional - product area for boost), limit (optional, default 5, max 10), min_score (optional - override threshold). Call before generating code or making decisions.")]
    async fn search_memory(
        &self,
        params: Parameters<SearchMemoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let result = tools::search_memory(
            params.0,
            self.embedder.as_ref().as_ref(),
            self.store.as_ref(),
            self.config.as_ref(),
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

    #[tool(description = "Get all memories for a user. Returns complete list with all metadata including superseded memories. Only call when user explicitly asks to see all memories.")]
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

    #[tool(description = "Correct an existing memory. Marks old memory as superseded and creates new corrected memory, inheriting repo/modules/lang/feature. New memory starts with reinforcement_count=2. Parameters: memory_id (required), user_id (required), correction (required - corrected content), topics (optional - new topics, inherits old if not provided), scope (optional - new scope, inherits old if not provided). Use when user corrects you instead of save_memory.")]
    async fn correct_memory(
        &self,
        params: Parameters<CorrectMemoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let result = tools::correct_memory(
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

    #[tool(description = "Get memory index showing existing topics, languages, repos, scope distribution, and recent memories. Returns: topics (sorted list of all topics used), langs (list of languages), repos (list of repositories), scopes (count by scope tier), total (non-superseded count), recent (last 3 memory summaries). Call at session start or before save_memory to see existing topics for consistency. LLM should prefer reusing existing topics when applicable. Parameters: user_id (required).")]
    async fn get_memory_index(
        &self,
        params: Parameters<GetMemoryIndexParams>,
    ) -> Result<CallToolResult, McpError> {
        let result = tools::get_memory_index(params.0, self.store.as_ref())
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
                "Memory server with LLM-first architecture. LLM pre-processes: extracts facts, determines topics, decides scope (global/lang/feature/repo/module). Server provides dumb storage with semantic search + boost-based ranking. Use search_memory before coding (passes context for boost). Use save_memory when patterns emerge (pass processed data with topics and scope)."
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
    dotenvy::dotenv().ok();
    let config = Config::from_env()?;

    let embedder = create_embedder(&config);

    let store = QdrantStore::new(
        &config.qdrant_host,
        config.qdrant_port,
        config.qdrant_collection.clone(),
        config.vector_size() as u64,
    )
    .await?;

    let service = MemoryServer::new(embedder, store, config)
        .serve(stdio())
        .await
        .inspect_err(|e| eprintln!("Error starting server: {}", e))?;

    service.waiting().await?;
    Ok(())
}
