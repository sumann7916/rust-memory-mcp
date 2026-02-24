use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router,
    transport::stdio,
    ErrorData as McpError, ServerHandler, ServiceExt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::process::Command;

#[derive(Clone)]
pub struct MemoryServer {
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SaveMemoryParams {
    pub content: String,
    pub user_id: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchMemoryParams {
    pub query: String,
    pub user_id: String,
    #[serde(default)]
    pub tags: Vec<String>,
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

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetTagsParams {
    pub user_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MemoryItem {
    pub id: String,
    pub memory: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub score: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SaveResult {
    pub success: bool,
    pub message: String,
    #[serde(default)]
    pub memories: Vec<MemoryItem>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchResult {
    pub results: Vec<MemoryItem>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AllMemoriesResult {
    pub memories: Vec<MemoryItem>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DeleteResult {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetTagsResult {
    pub tags: Vec<String>,
}

async fn call_python(operation: &str, payload: &str) -> Result<String, String> {
    // Get the directory where the binary is located
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get executable path: {}", e))?;
    let exe_dir = exe_path.parent()
        .ok_or("Failed to get executable directory")?;
    
    // Binary is at: rust-mem/mcp/target/release/memory-mcp
    // We need to get to: rust-mem/
    // So go up 3 levels: release -> target -> mcp -> rust-mem
    let project_root = exe_dir
        .parent()  // target
        .ok_or("Failed to get target directory")?
        .parent()  // mcp
        .ok_or("Failed to get mcp directory")?
        .parent()  // rust-mem
        .ok_or("Failed to get rust-mem directory")?;
    
    let memory_script = project_root.join("memory.py");
    
    if !memory_script.exists() {
        return Err(format!("memory.py not found at: {}", memory_script.display()));
    }
    
    // Use venv python if available, otherwise fall back to python3
    let venv_python = project_root.join("venv/bin/python3");
    let python_cmd = if venv_python.exists() {
        venv_python.to_string_lossy().to_string()
    } else {
        "python3".to_string()
    };
    
    let output = Command::new(&python_cmd)
        .arg(&memory_script)
        .arg(operation)
        .arg(payload)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to execute memory.py: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("memory.py failed: {}", stderr));
    }

    String::from_utf8(output.stdout).map_err(|e| format!("Invalid UTF-8 output: {}", e))
}

#[tool_router]
impl MemoryServer {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Save a memory with tags for context scoping. Tags can be: 'repo:NAME' (specific repo), 'module:PATH' (specific file/directory), 'lang:LANGUAGE', 'category:TYPE' (style/pattern/bug_fix/preference). Example tags: ['repo:rust-mem', 'lang:rust', 'category:pattern']. IMPORTANT: DO NOT use 'global' tag - the system will automatically determine if something is truly global based on context. Use specific, contextual tags instead. WHEN TO CALL: Call proactively when you solve a problem, implement a successful solution, fix a bug, or learn a new pattern. Save lessons learned, working solutions, and user preferences automatically. Don't wait for explicit confirmation - if something works or you learn something useful, save it.")]
    async fn save_memory(
        &self,
        params: Parameters<SaveMemoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let payload = serde_json::to_string(&params.0).map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: format!("Failed to serialize params: {}", e).into(),
            data: None,
        })?;

        let result = call_python("save", &payload).await.map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: e.into(),
            data: None,
        })?;

        let save_result: SaveResult = serde_json::from_str(&result).map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: format!("Failed to parse response: {} - raw: {}", e, result).into(),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&save_result).unwrap_or(result),
        )]))
    }

    #[tool(description = "Search memories by query and filter by tags. Pass relevant tags for current context (e.g., ['global', 'repo:rust-mem', 'lang:rust']) to get scoped results. Memories matching ANY tag will be returned. Always include 'global' tag to get universal patterns. WHEN TO CALL: ALWAYS call this BEFORE generating code, implementing features, or answering technical questions. This is a proactive tool - use it to check for relevant lessons, patterns, and context. DO NOT wait for user to ask.")]
    async fn search_memory(
        &self,
        params: Parameters<SearchMemoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let payload = serde_json::to_string(&params.0).map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: format!("Failed to serialize params: {}", e).into(),
            data: None,
        })?;

        let result = call_python("search", &payload).await.map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: e.into(),
            data: None,
        })?;

        let search_result: SearchResult = serde_json::from_str(&result).map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: format!("Failed to parse response: {} - raw: {}", e, result).into(),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&search_result).unwrap_or(result),
        )]))
    }

    #[tool(description = "Get all memories stored for a user. Returns all memories with their content and tags. WHEN TO CALL: Only when user explicitly asks to see/list/review all memories, or when you need full memory details (not just tags). For discovering available tags, use get_tags instead as it's more efficient.")]
    async fn get_all_memories(
        &self,
        params: Parameters<GetAllMemoriesParams>,
    ) -> Result<CallToolResult, McpError> {
        let payload = serde_json::to_string(&params.0).map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: format!("Failed to serialize params: {}", e).into(),
            data: None,
        })?;

        let result = call_python("get_all", &payload).await.map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: e.into(),
            data: None,
        })?;

        let all_result: AllMemoriesResult = serde_json::from_str(&result).map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: format!("Failed to parse response: {} - raw: {}", e, result).into(),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&all_result).unwrap_or(result),
        )]))
    }

    #[tool(description = "Delete a specific memory by its ID. WHEN TO CALL: Only when user explicitly asks to delete, remove, or forget a specific memory. Requires the memory ID. DO NOT call proactively or suggest deletion unless user clearly wants it.")]
    async fn delete_memory(
        &self,
        params: Parameters<DeleteMemoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let payload = serde_json::to_string(&params.0).map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: format!("Failed to serialize params: {}", e).into(),
            data: None,
        })?;

        let result = call_python("delete", &payload).await.map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: e.into(),
            data: None,
        })?;

        let delete_result: DeleteResult = serde_json::from_str(&result).map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: format!("Failed to parse response: {} - raw: {}", e, result).into(),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&delete_result).unwrap_or(result),
        )]))
    }

    #[tool(description = "Get all unique tags from stored memories. WHEN TO CALL: Call proactively at the start of a coding session to discover what tags/contexts exist. Use the returned tags to make informed search_memory calls with relevant tags. This is lightweight compared to get_all_memories.")]
    async fn get_tags(
        &self,
        params: Parameters<GetTagsParams>,
    ) -> Result<CallToolResult, McpError> {
        let payload = serde_json::to_string(&params.0).map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: format!("Failed to serialize params: {}", e).into(),
            data: None,
        })?;

        let result = call_python("get_tags", &payload).await.map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: e.into(),
            data: None,
        })?;

        let tags_result: GetTagsResult = serde_json::from_str(&result).map_err(|e| McpError {
            code: ErrorCode::INTERNAL_ERROR,
            message: format!("Failed to parse response: {} - raw: {}", e, result).into(),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&tags_result).unwrap_or(result),
        )]))
    }
}

#[tool_handler]
impl ServerHandler for MemoryServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Memory server for storing and retrieving coding lessons, facts, and context. \
                 Use get_tags at session start to discover available tags/contexts. \
                 Use search_memory before generating code to check for relevant past lessons. \
                 Use save_memory proactively when solutions work, bugs are fixed, or patterns are learned - don't wait for user to tell you."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = MemoryServer::new()
        .serve(stdio())
        .await
        .inspect_err(|e| eprintln!("Error starting server: {}", e))?;

    service.waiting().await?;
    Ok(())
}
