# Memory MCP Server

A Rust MCP (Model Context Protocol) server that provides persistent memory capabilities using Qdrant vector database. The server implements an LLM-first architecture where the LLM handles all intelligence (fact extraction, topic determination, scope decisions) and the server provides dumb storage with semantic search + boost-based ranking.

## Architecture

```
┌─────────────────┐     stdio      ┌─────────────────┐
│   AI Client     │◄──────────────►│  Rust MCP       │
│  (e.g. Cursor)  │   JSON-RPC     │  Server         │
└─────────────────┘                └────────┬────────┘
                                            │
                                            ▼
                                   ┌─────────────────┐
                                   │  Qdrant Vector  │
                                   │  Database       │
                                   └────────┬────────┘
                                            │
                                   ┌────────┴────────┐
                                   │  Embedder Model │
                                   │  (local/cloud)  │
                                   └─────────────────┘
```

## Core Philosophy

**LLM = Intelligence**: The LLM does all the thinking - extracting facts, determining topics, deciding scope.

**Server = Dumb Storage + Boost Ranking**: The server just does semantic search, applies boost multipliers, and returns ranked results.

**Only Hard Filter**: `user_id` - Everything else (repo, lang, module, feature, topics, scope) becomes boost signals.

## Prerequisites

- Rust (for building the MCP server)
- One of:
  - Ollama running locally (default)
  - Gemini API key
  - OpenAI API key
- Qdrant (for vector storage)

## Installation

### 1. Install Qdrant

```bash
# Option A: Run with Docker
docker run -p 6334:6334 qdrant/qdrant

# Option B: Install locally
# See https://qdrant.tech/documentation/quick-start/
```

### 2. Build the Rust MCP server

```bash
cd mcp
cargo build --release
```

The binary will be at `mcp/target/release/memory-mcp`.

### 3. Set up your embedder provider

#### Option A: Local with Ollama (default)

```bash
# Install Ollama: https://ollama.ai
# Start Ollama
ollama serve

# Pull embedding model
ollama pull nomic-embed-text
```

No environment variables needed - Ollama is the default.

#### Option B: Gemini (cloud)

```bash
export EMBEDDER_PROVIDER=gemini
export EMBEDDER_MODEL=models/text-embedding-004
export GEMINI_API_KEY=your-api-key
```

#### Option C: OpenAI (cloud)

```bash
export EMBEDDER_PROVIDER=openai
export EMBEDDER_MODEL=text-embedding-3-small
export OPENAI_API_KEY=your-api-key
```

### Testing the memory-chat agent locally (Gemini)

You can run the memory-chat agent in a local REPL using your Gemini API key. The agent uses the same tools (search_memory, save_memory, get_memory_index) and talks to Qdrant.

1. **Start Qdrant** (if not already running):

   ```bash
   docker run -p 6334:6334 qdrant/qdrant
   ```

2. **Run the agent binary** from the `mcp` directory:

   ```bash
   cd mcp
   GEMINI_API_KEY=your_gemini_key \
   LLM_PROVIDER=gemini \
   EMBEDDER_PROVIDER=gemini \
   AGENT_PROVIDER=gemini \
   USER_ID=sumankhadka \
   cargo run --bin memory_chat_agent
   ```

   Type a message and press Enter; the agent will reply (and may call tools). Empty line exits.

## Configuration

All configuration is done via environment variables:

### Embedder

| Variable | Description | Default |
|----------|-------------|---------|
| `EMBEDDER_PROVIDER` | `ollama`, `gemini`, `openai` | `ollama` |
| `EMBEDDER_MODEL` | Model name for embeddings | Provider-dependent |
| `GEMINI_API_KEY` | Gemini API key | - |
| `OPENAI_API_KEY` | OpenAI API key | - |
| `OLLAMA_BASE_URL` | Ollama endpoint | `http://localhost:11434` |

### Vector Store (Qdrant)

| Variable | Description | Default |
|----------|-------------|---------|
| `QDRANT_HOST` | Qdrant host | `localhost` |
| `QDRANT_PORT` | Qdrant port | `6334` |
| `QDRANT_COLLECTION` | Collection name | `coding_memories` |

### Boost Multipliers

| Variable | Description | Default |
|----------|-------------|---------|
| `SCOPE_BOOST_LANG` | Boost for lang-scoped memories | `1.3` |
| `SCOPE_BOOST_FEATURE` | Boost for feature-scoped memories | `1.4` |
| `SCOPE_BOOST_REPO` | Boost for repo-scoped memories | `1.6` |
| `SCOPE_BOOST_MODULE` | Boost for module-scoped memories | `2.0` |
| `TOPIC_BOOST_MAX` | Max additional boost from topic overlap | `0.3` |
| `MEMORY_SCORE_THRESHOLD` | Min final score to return | `0.75` |
| `MEMORY_DEDUP_THRESHOLD` | Similarity threshold for deduplication | `0.90` |

## MCP Tools

The server exposes six tools:

### `get_memory_index`

Get an index of all existing memories to help with topic consistency.

**Use case:** Call at session start to see what topics, languages, and repos already exist. Cache the result and use it when determining topics for new memories.

**Parameters:**
- `user_id` (string, required): User identifier

**Response:**
```json
{
  "topics": ["auth", "jwt", "security", "payments", "async", "error-handling"],
  "langs": ["rust", "typescript", "python"],
  "repos": ["rust-mem", "portpro-backend"],
  "scopes": {
    "global": 12,
    "lang": 8,
    "feature": 6,
    "repo": 14,
    "module": 4
  },
  "total": 44,
  "recent": [
    "[feature] JWT refresh tokens should be rotated on every use...",
    "[global] Use Result type over panic for error handling in Rust",
    "[repo] API uses PostgreSQL connection pool with max 20 connections"
  ]
}
```

**Benefits:**
- **Topic consistency**: Reuse existing topics instead of creating variants ("auth" vs "authentication")
- **Context awareness**: See what languages/repos have memories
- **Scope distribution**: Understand how memories are organized
- **Session caching**: Call once, reuse throughout session

**Example:**
```javascript
// At session start
const index = await get_memory_index({ user_id: "sumankhadka" });
// Cache topics: ["auth", "jwt", "security", ...]

// Later when saving
// User says: "JWT tokens should expire"
// LLM checks cached topics → reuses ["auth", "jwt", "security"]
// Instead of creating new: ["authentication", "json-web-tokens", "security"]
```

### `save_memory`

Save a memory. **LLM must pre-process before calling.**

**LLM Pre-processing Steps:**
1. Extract clean single-sentence fact
2. Determine 2-5 topics from conversation context
3. Decide scope using decision tree (see below)

**Parameters:**
- `content` (string, required): Pre-processed fact
- `user_id` (string, required): User identifier
- `topics` (array of strings, required): Topic tags
- `scope` (string, required): One of: `global`, `lang`, `feature`, `repo`, `module`
- `repo` (string, optional): Repository name
- `lang` (string, optional): Programming language
- `module` (string, optional): Directory path (e.g., `src/auth`)
- `feature` (string, optional): Product area (e.g., `invoicing`)

**Scope Decision Tree:**

```
Would I want this in EVERY project I ever work on?           → global
Would I want this in every [lang] project?                   → lang
Would I want this whenever working on [feature/domain]?      → feature
Is this specific to this one repo's architecture/decisions?  → repo
Is this specific to this one file/folder?                    → module
```

**Example:**

```javascript
// User says: "JWT refresh tokens should be rotated on every use"

// LLM thinks:
// → Fact: "JWT refresh tokens should be rotated on every use and old token invalidated to prevent replay attacks"
// → Topics: ["auth", "jwt", "security", "tokens"]
// → Scope: "feature" (applies to auth feature across projects)

save_memory({
  content: "JWT refresh tokens should be rotated on every use and old token invalidated to prevent replay attacks",
  user_id: "sumankhadka",
  topics: ["auth", "jwt", "security", "tokens"],
  scope: "feature",
  lang: "typescript",
  feature: "auth"
})
```

### `search_memory`

Search memories with semantic similarity + boost-based ranking.

**Parameters:**
- `query` (string, required): Search query
- `user_id` (string, required): User identifier
- `repo` (string, optional): Repository name (for boost)
- `module` (string, optional): Directory path (for boost)
- `lang` (string, optional): Programming language (for boost)
- `feature` (string, optional): Product area (for boost)
- `limit` (number, optional): Max results (default: 5, max: 10)
- `min_score` (number, optional): Override score threshold

**How Search Works:**
1. Qdrant semantic search (filters only by `user_id` and `superseded: false`)
2. For each result, calculate boost based on:
   - Scope match (module=2.0x, repo=1.6x, feature=1.4x, lang=1.3x, global=1.0x)
   - Topic overlap (up to +0.3x)
   - Confidence (high=1.2x, medium=1.0x, low=0.8x)
3. Final score = semantic_score × boost
4. Sort by final score, return top N

**Response includes:**
- `content`: Memory text
- `scope`: Scope tier
- `topics`: Topic tags
- `score`: Semantic similarity score
- `boost`: Applied boost multiplier
- `confidence`: low/medium/high
- Context fields: `repo`, `lang`, `modules`, `feature`

**Example:**

```javascript
search_memory({
  query: "JWT token handling and refresh strategy",
  user_id: "sumankhadka",
  repo: "rust-mem",
  lang: "rust",
  module: "src/auth",
  limit: 5
})

// Returns memories ranked by semantic_score * boost
// Global-scoped memories always surface (boost=1.0)
// Module-matching memories get biggest boost (boost=2.0)
```

### `get_all_memories`

Get all memories for a user, including superseded ones.

**Parameters:**
- `user_id` (string, required): User identifier

**Only call when user explicitly asks to see all memories.**

### `delete_memory`

Delete a specific memory by ID.

**Parameters:**
- `memory_id` (string, required): Memory ID to delete
- `user_id` (string, required): User identifier

### `correct_memory`

Correct an existing memory. Marks old as superseded, creates new with correction.

**Parameters:**
- `memory_id` (string, required): Memory ID to correct
- `user_id` (string, required): User identifier
- `correction` (string, required): Corrected content
- `topics` (array of strings, optional): New topics (inherits old if not provided)
- `scope` (string, optional): New scope (inherits old if not provided)

**Note:** New memory starts with `reinforcement_count: 2` (correction implies confirmation).

## Usage with Cursor

Add to your Cursor MCP settings (`.cursor/mcp.json` or global settings):

```json
{
  "mcpServers": {
    "memory": {
      "command": "/path/to/rust-mem/mcp/target/release/memory-mcp",
      "env": {
        "EMBEDDER_PROVIDER": "ollama"
      }
    }
  }
}
```

## Memory Behavior

### Scope Tiers

Memories are organized into 5 scope tiers that determine boost multipliers:

| Scope | When to Use | Boost Multiplier |
|-------|-------------|------------------|
| `global` | Universal patterns that apply everywhere | 1.0x (always surfaces) |
| `lang` | Language-specific patterns | 1.3x (when lang matches) |
| `feature` | Cross-repo domain patterns (auth, payments, etc.) | 1.4x (when feature matches) |
| `repo` | Repository-specific architecture/decisions | 1.6x (when repo matches) |
| `module` | File/directory specific details | 2.0x (when module matches) |

**Non-matching scope gets 0.5x penalty** - but memories can still surface if semantic score is high enough.

### Topics

Free-form tags extracted by LLM from conversation context:

- **Technical**: `auth`, `jwt`, `websockets`, `database`, `async`, `error-handling`
- **Domain**: `payments`, `invoicing`, `notifications`, `file-upload`
- **Tools**: `tokio`, `serde`, `docker`, `kubernetes`

Topics contribute to boost through overlap scoring (up to +0.3x).

### Confidence Levels

Based on reinforcement count:

- **low** (count: 1): New memory, treat as hint, boost=0.8x
- **medium** (count: 2-4): Confirmed pattern, boost=1.0x
- **high** (count: 5+): Established pattern, boost=1.2x

### Deduplication

Memories with >0.90 semantic similarity automatically reinforce existing memory instead of creating duplicate. Reinforcement:
- Increments reinforcement_count
- Merges content if different
- Combines topics

### Contradiction Detection

Memories with 0.75-0.90 similarity are checked for contradictions. If new memory contradicts old, the old is marked as superseded.

## Example Workflows

### Save a Preference

```javascript
// User: "I prefer using Result over panic for error handling"

// LLM pre-processes:
// Fact: "Use Result type over panic for error handling in Rust"
// Topics: ["rust", "error-handling", "patterns"]
// Scope: "lang" (applies to all Rust projects)

save_memory({
  content: "Use Result type over panic for error handling in Rust",
  user_id: "sumankhadka",
  topics: ["rust", "error-handling", "patterns"],
  scope: "lang",
  lang: "rust"
})
```

### Save a Bug Fix

```javascript
// User: "The connection timeout was caused by missing keep-alive"

// LLM pre-processes:
// Fact: "Connection timeout errors caused by missing TCP keep-alive settings — fixed by adding TCP_KEEPALIVE to socket options"
// Topics: ["networking", "timeout", "tcp", "debugging"]
// Scope: "feature" (applies to networking code across projects)

save_memory({
  content: "Connection timeout errors caused by missing TCP keep-alive settings — fixed by adding TCP_KEEPALIVE to socket options",
  user_id: "sumankhadka",
  topics: ["networking", "timeout", "tcp", "debugging"],
  scope: "feature",
  feature: "networking"
})
```

### Search Before Coding

```javascript
// Before implementing JWT auth in rust-mem/src/auth

search_memory({
  query: "JWT token handling best practices",
  user_id: "sumankhadka",
  repo: "rust-mem",
  module: "src/auth",
  lang: "rust",
  limit: 5
})

// Returns:
// 1. Global JWT patterns (boost=1.0)
// 2. Rust-specific JWT implementations (boost=1.3)
// 3. Auth feature memories (boost=1.4)
// 4. rust-mem repo decisions (boost=1.6)
// All ranked by semantic_score * boost
```

## Breaking Changes from v1.x

⚠️ **This is v2.0.0 with breaking changes:**

- Old memory format incompatible (`category` → `scope` + `topics`)
- Existing memories cannot be migrated (scope/topic inference unreliable)
- Users must rebuild their memory banks with new system
- LLM pre-processing now required for `save_memory`
- Removed tools: `get_preferences`, `consolidate_memories`, `get_memory_stats`
- No LLM provider needed (server only uses embedder)

## Recommended Usage Pattern

1. **Before generating code**: Call `search_memory` with relevant context to get applicable patterns
2. **When something works**: Call `save_memory` with pre-processed fact, topics, and scope
3. **On corrections**: Call `correct_memory` to supersede incorrect information
4. **Trust the code**: If memory contradicts what you see in code, trust the code

## Key Principles

✅ **LLM is smart**: Does all reasoning, extraction, classification  
✅ **Server is dumb**: Just search, boost, rank, store  
✅ **Only hard filter**: `user_id`  
✅ **Everything else boosts**: repo, lang, module, feature, topics, scope  
✅ **Memories cross boundaries**: repo-scoped memories can help in other repos via lower boost  
✅ **Simple API**: LLM does work upfront, server execution is straightforward
