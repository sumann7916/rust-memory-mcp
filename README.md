# Memory MCP Server

A Rust MCP (Model Context Protocol) server that provides persistent memory capabilities using mem0. The server exposes tools for saving, searching, retrieving, and deleting memories, with support for multiple LLM providers.

## Architecture

```
┌─────────────────┐     stdio      ┌─────────────────┐
│   AI Client     │◄──────────────►│  Rust MCP       │
│  (e.g. Cursor)  │   JSON-RPC     │  Server         │
└─────────────────┘                └────────┬────────┘
                                            │ subprocess
                                            ▼
                                   ┌─────────────────┐
                                   │   memory.py     │
                                   │   (mem0)        │
                                   └────────┬────────┘
                                            │
                          ┌─────────────────┼─────────────────┐
                          ▼                 ▼                 ▼
                   ┌───────────┐     ┌───────────┐     ┌───────────┐
                   │  Ollama   │     │  Gemini   │     │  OpenAI   │
                   │  (local)  │     │  (cloud)  │     │  (cloud)  │
                   └───────────┘     └───────────┘     └───────────┘
```

## Prerequisites

- Rust (for building the MCP server)
- Python 3.10+
- One of:
  - Ollama running locally (default)
  - Gemini API key
  - OpenAI API key

## Installation

### 1. Install Python dependencies

```bash
pip install -r requirements.txt
```

### 2. Build the Rust MCP server

```bash
cd mcp
cargo build --release
```

The binary will be at `mcp/target/release/memory-mcp`.

### 3. Set up your LLM provider

#### Option A: Local with Ollama (default)

```bash
# Install Ollama: https://ollama.ai
# Start Ollama
ollama serve

# Pull required models
ollama pull qwen2.5-coder:7b
ollama pull nomic-embed-text
```

No environment variables needed - Ollama is the default.

#### Option B: Gemini (cloud)

```bash
export LLM_PROVIDER=gemini
export LLM_MODEL=gemini-2.0-flash
export GEMINI_API_KEY=your-api-key
export EMBEDDER_PROVIDER=gemini
export EMBEDDER_MODEL=models/text-embedding-004
```

#### Option C: OpenAI (cloud)

```bash
export LLM_PROVIDER=openai
export LLM_MODEL=gpt-4o-mini
export OPENAI_API_KEY=your-api-key
export EMBEDDER_PROVIDER=openai
export EMBEDDER_MODEL=text-embedding-3-small
```

## Configuration

All configuration is done via environment variables:

### LLM & Embedder

| Variable | Description | Default |
|----------|-------------|---------|
| `LLM_PROVIDER` | `ollama`, `gemini`, `openai`, `anthropic` | `ollama` |
| `LLM_MODEL` | Model name for the LLM | Provider-dependent |
| `EMBEDDER_PROVIDER` | `ollama`, `gemini`, `openai` | `ollama` |
| `EMBEDDER_MODEL` | Model name for embeddings | Provider-dependent |
| `GEMINI_API_KEY` | Gemini API key | - |
| `OPENAI_API_KEY` | OpenAI API key | - |
| `ANTHROPIC_API_KEY` | Anthropic API key | - |
| `OLLAMA_BASE_URL` | Ollama endpoint | `http://localhost:11434` |

### Vector Store (ChromaDB)

| Variable | Description | Default |
|----------|-------------|---------|
| `CHROMA_HOST` | ChromaDB host (enables Docker mode) | - (local file storage) |
| `CHROMA_PORT` | ChromaDB port | `8000` |
| `CHROMA_COLLECTION` | Collection name | `coding_memories` |

You can mix providers (e.g., Ollama for embeddings, OpenAI for LLM).

### Using ChromaDB with Docker

For persistent storage with Docker:

```bash
# Start ChromaDB
docker compose up -d

# Configure the MCP server to use it
export CHROMA_HOST=localhost
export CHROMA_PORT=8000
```

Without `CHROMA_HOST` set, the system uses local file storage at `./chroma_db`.

## MCP Tools

The server exposes four tools with tag-based memory scoping:

### `save_memory`
Save a memory with tags for context scoping.

**Parameters:**
- `content` (string): The memory content to save
- `user_id` (string): User identifier for scoping memories
- `tags` (array of strings, optional): Tags for organizing memories

**Tag conventions:**

**Scope tags:**
- `global` - Universal patterns/preferences that apply everywhere
- `repo:NAME` - Repository-specific (e.g., `repo:rust-mem`)
- `module:PATH` - File/directory-specific (e.g., `module:src/auth`)
- `project:NAME` - Project-wide patterns

**Context tags:**
- `lang:LANGUAGE` - Language-specific (e.g., `lang:rust`, `lang:python`)
- `framework:NAME` - Framework patterns (e.g., `framework:tokio`, `framework:react`)

**Category tags:**
- `category:style` - Coding style preferences
- `category:pattern` - Design patterns
- `category:bug_fix` - Known issues and fixes
- `category:preference` - Tool/library preferences
- `category:api_usage` - API usage examples

**Example:**
```javascript
save_memory({
  content: "rmcp 0.16 requires schemars 1.x not 0.8.x to avoid version conflicts",
  user_id: "suman",
  tags: ["repo:rust-mem", "lang:rust", "category:bug_fix"]
})
```

### `search_memory`
Search memories semantically with tag filtering.

**Parameters:**
- `query` (string): Search query
- `user_id` (string): User identifier
- `tags` (array of strings, optional): Filter by tags (OR logic - matches ANY tag)
- `limit` (number, optional): Max results (default: 5)

**Tag filtering:** Memories matching ANY of the provided tags will be returned. Always include `global` to get universal patterns.

**Example:**
```javascript
// When working in rust-mem/mcp/src/main.rs
search_memory({
  query: "how to handle MCP tool parameters",
  user_id: "suman",
  tags: ["global", "repo:rust-mem", "module:mcp/src", "lang:rust"],
  limit: 5
})
```

### `get_all_memories`
Get all memories stored for a user, including their tags.

**Parameters:**
- `user_id` (string): User identifier

### `delete_memory`
Delete a specific memory by its ID.

**Parameters:**
- `memory_id` (string): The memory ID to delete
- `user_id` (string): User identifier

## Usage with Cursor

Add to your Cursor MCP settings (`.cursor/mcp.json`):

```json
{
  "mcpServers": {
    "memory": {
      "command": "/path/to/rust-mem/mcp/target/release/memory-mcp",
      "env": {
        "LLM_PROVIDER": "ollama"
      }
    }
  }
}
```

## Memory Behavior

- All memories are scoped by `user_id` and `agent_id="coding-agent"`
- Memories can be tagged with flexible context tags for better organization
- Tags support OR logic: searching with `["lang:rust", "category:pattern"]` returns memories matching either tag
- Memories are stored locally in `./chroma_db` (or Docker ChromaDB if `CHROMA_HOST` is set)
- The LLM extracts facts from your input when saving
- Search uses semantic similarity (not keyword matching) with optional tag filtering

## Tag-Based Memory Strategy

**When saving memories:**
1. Use `global` for universal patterns that apply everywhere
2. Add `repo:NAME` for repository-specific knowledge
3. Add `lang:LANGUAGE` for language-specific patterns
4. Use category tags to classify the type of memory

**When searching memories:**
1. Always include `global` to get universal patterns
2. Add current repo/module tags for context-specific results
3. Add language tag for language-specific patterns
4. Results include ALL tags, ordered by relevance

**Example workflow:**
```javascript
// Working in rust-mem/mcp/src/main.rs

// Search with context
search_memory({
  query: "handling async errors",
  user_id: "suman",
  tags: ["global", "repo:rust-mem", "lang:rust", "category:pattern"]
})

// Save with tags
save_memory({
  content: "Use .await? for async error propagation in Rust",
  user_id: "suman",
  tags: ["global", "lang:rust", "category:pattern"]
})
```

## Recommended Usage Pattern

1. **Before generating code**: Call `search_memory` with relevant tags to check for applicable patterns
2. **When something works**: Call `save_memory` with appropriate tags to remember the solution
3. **On corrections**: Save what was wrong, what fixed it, with tags indicating scope and category
