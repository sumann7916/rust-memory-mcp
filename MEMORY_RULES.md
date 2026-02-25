# Memory MCP Integration Rules

You have access to a memory system via the `user-user-memory` MCP server with an LLM-first architecture.

## Core Philosophy

**You = Intelligence** (you do all the thinking)  
**Server = Storage + Boost Ranking** (dumb storage with semantic search)

## Available Tools

- `get_memory_index` - Get existing topics, langs, repos for consistency
- `save_memory` - Save new memories (you pre-process first)
- `search_memory` - Search memories by semantic similarity + boost ranking
- `get_all_memories` - List all memories for a user
- `delete_memory` - Delete a specific memory
- `correct_memory` - Correct an existing memory (supersedes old, creates new)

## At Session Start

**ALWAYS call `get_memory_index` first:**

```javascript
get_memory_index({ user_id: "sumankhadka" })
```

This returns:
- **topics**: List of all existing topics (for consistency)
- **langs**: Languages in use
- **repos**: Repositories with memories
- **scopes**: Distribution by scope tier
- **total**: Total non-superseded memories
- **recent**: Last 3 memory summaries

**Cache this for the session.** Use it when determining topics for new memories.

## Before Calling save_memory

**YOU MUST PRE-PROCESS**. Ask yourself:

1. **Extract the fact**: What is the single most important fact?
   - Make it one clear, concise sentence
   - Remove fluff, focus on the core insight

2. **Determine topics**: What 2-5 topics does this touch? Use full conversation context.
   - **Check cached topics from get_memory_index**
   - **Reuse existing topics when applicable** (e.g., use "auth" not "authentication" if "auth" exists)
   - Only create new topics if existing ones don't fit
   - Technical: `auth`, `jwt`, `websockets`, `database`, `async`, `error-handling`
   - Domain: `payments`, `invoicing`, `notifications`, `file-upload`
   - Tools: `tokio`, `serde`, `docker`, `kubernetes`
   - Keep topics specific and searchable

3. **Decide scope** - Use this decision tree:
   ```
   Would I want this in EVERY project I ever work on?           → global
   Would I want this in every [lang] project?                   → lang
   Would I want this whenever working on [feature/domain]?      → feature
   Is this specific to this one repo's architecture/decisions?  → repo
   Is this specific to this one file/folder?                    → module
   ```

## When to Save

Call `save_memory` when:
- User shows a preference or rule
- You solve a non-obvious problem
- A pattern emerges that would help in the future
- Something is worth remembering for later

**When user corrects you**: Use `correct_memory` instead of `save_memory`.

## When to Search

**BEFORE any of these actions**, call `search_memory`:
- Responding to coding questions
- Generating or reviewing code
- Creating implementation plans
- Making architectural decisions
- Debugging or fixing issues
- Suggesting refactors

## How Search Works

Server returns memories ranked by: **semantic_score × boost**

### Boost Calculation

```
Scope boost (base multiplier):
- global:  1.0x (always surfaces, no penalty)
- lang:    1.3x if lang matches
- feature: 1.4x if feature matches
- repo:    1.6x if repo matches
- module:  2.0x if module matches
- Non-matching: 0.5x (penalty)

+ Topic overlap boost: up to +0.3x

+ Confidence boost:
  - high (5+ reinforcements): 1.2x
  - medium (2-4):             1.0x
  - low (1):                  0.8x

= Final score
```

**Key points:**
- Global-scoped memories always surface (boost = 1.0)
- Module-matching memories get biggest boost (2.0x)
- Non-matching scope still surfaces if semantic score is high enough
- Repo-scoped memories can help in other repos (lower boost)

## API Usage

### save_memory

```javascript
save_memory({
  content: "JWT refresh tokens should be rotated on every use and old token invalidated to prevent replay attacks",
  user_id: "sumankhadka",
  topics: ["auth", "jwt", "security", "tokens"],
  scope: "feature",
  lang: "typescript",
  feature: "auth"
})
```

**Required:**
- `content`: Pre-processed fact (one clear sentence)
- `user_id`: "sumankhadka"
- `topics`: Array of 2-5 topic strings
- `scope`: One of: `global`, `lang`, `feature`, `repo`, `module`

**Optional context (for scoping):**
- `repo`: Repository name
- `lang`: Programming language
- `module`: Directory path (e.g., `src/auth`)
- `feature`: Product area (e.g., `invoicing`)

### search_memory

```javascript
search_memory({
  query: "JWT token handling and refresh strategy",
  user_id: "sumankhadka",
  repo: "rust-mem",
  lang: "rust",
  module: "src/auth",
  limit: 5
})
```

**Required:**
- `query`: Descriptive search query
- `user_id`: "sumankhadka"

**Optional context (for boost):**
- `repo`: Current repository
- `module`: Current directory path
- `lang`: Current language
- `feature`: Current product area
- `limit`: Max results (default: 5, max: 10)
- `min_score`: Override threshold (optional)

**Response fields:**
- `content`: Memory text
- `scope`: Scope tier
- `topics`: Topic tags
- `score`: Semantic similarity
- `boost`: Applied boost multiplier
- `confidence`: low/medium/high

### correct_memory

```javascript
correct_memory({
  memory_id: "abc-123",
  user_id: "sumankhadka",
  correction: "Actually use 15min access tokens not 1hr — shorter window reduces replay risk",
  topics: ["auth", "jwt", "security"],  // optional
  scope: "feature"  // optional
})
```

## Scope Tiers Explained

### global
- **When**: Universal patterns that apply everywhere
- **Examples**: "Avoid else blocks, use early returns", "Prefer functional patterns over loops"
- **Boost**: 1.0x (always surfaces)

### lang
- **When**: Language-specific patterns
- **Examples**: "Use Result type over panic in Rust", "Prefer async/await over callbacks in TypeScript"
- **Boost**: 1.3x when lang matches

### feature
- **When**: Cross-repo domain patterns
- **Examples**: "JWT refresh tokens should be rotated", "Upload files to S3 with pre-signed URLs"
- **Boost**: 1.4x when feature matches

### repo
- **When**: Repository-specific architecture/decisions
- **Examples**: "Use PostgreSQL connection pool with max 20 connections", "API uses RESTful endpoints with /api/v1 prefix"
- **Boost**: 1.6x when repo matches

### module
- **When**: File/directory specific details
- **Examples**: "auth.rs uses Argon2 for password hashing", "index.ts exports all components"
- **Boost**: 2.0x when module matches

## Topic Examples

**Good topics** (specific and searchable):
- `["auth", "jwt", "security"]`
- `["database", "postgres", "connection-pooling"]`
- `["error-handling", "async", "tokio"]`
- `["payments", "stripe", "webhooks"]`

**Bad topics** (too vague):
- `["code", "programming", "software"]`
- `["best-practices", "patterns"]`

## Confidence Levels

Memories have confidence based on reinforcement count:

- **low** (count: 1): New memory, treat as hint only
- **medium** (count: 2-4): Confirmed pattern
- **high** (count: 5+): Established pattern

**Trust higher confidence memories more** - they've been reinforced through repeated use.

## Automatic Features

### Deduplication
- Memories with >0.90 similarity reinforce existing instead of creating duplicate
- Reinforcement increments count, merges content, combines topics

### Contradiction Detection
- Memories with 0.75-0.90 similarity checked for contradictions
- New memory supersedes old if contradiction detected

## Important Notes

- **No get_preferences tool**: Global memories always surface in search
- **Trust the code**: If memory contradicts code, trust the code
- **Memories are hints**: They inform your approach, not facts to blindly follow
- **Save proactively**: Don't wait for explicit requests
- **Pre-process first**: Never pass raw user input to save_memory

## Example Workflows

### Session Start

```javascript
// First call of the session
get_memory_index({ user_id: "sumankhadka" })

// Response:
{
  topics: ["auth", "jwt", "security", "payments", "async", "error-handling"],
  langs: ["rust", "typescript"],
  repos: ["rust-mem", "portpro-backend"],
  scopes: { global: 12, lang: 8, feature: 6, repo: 14, module: 4 },
  total: 44,
  recent: [...]
}

// Cache these topics for the session
```

### Save a Preference

```javascript
// User: "I prefer using Result over panic for error handling"

// Check cached topics: ["auth", "jwt", "security", "payments", "async", "error-handling"]
// "error-handling" exists → reuse it
// Also relevant: rust patterns

// Your thinking:
// Fact: "Use Result type over panic for error handling in Rust"
// Topics: ["rust", "error-handling", "patterns"] (reuse "error-handling" from cache)
// Scope: "lang" (applies to all Rust projects)

save_memory({
  content: "Use Result type over panic for error handling in Rust",
  user_id: "sumankhadka",
  topics: ["rust", "error-handling", "patterns"],
  scope: "lang",
  lang: "rust"
})
```

### Save with New Topic

```javascript
// User: "Use rate limiting on all public API endpoints"

// Check cached topics: no "rate-limiting" exists
// This is a new topic → okay to create it

save_memory({
  content: "Use rate limiting on all public API endpoints",
  user_id: "sumankhadka",
  topics: ["api", "rate-limiting", "security"],  // New: "rate-limiting"
  scope: "feature",
  feature: "api"
})
```

### Save a Bug Fix

```javascript
// User: "I prefer using Result over panic for error handling"

// Your thinking:
// Fact: "Use Result type over panic for error handling in Rust"
// Topics: ["rust", "error-handling", "patterns"]
// Scope: lang (applies to all Rust projects)

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

// Your thinking:
// Fact: "Connection timeout errors caused by missing TCP keep-alive settings — fixed by adding TCP_KEEPALIVE to socket options"
// Topics: ["networking", "timeout", "tcp", "debugging"]
// Scope: feature (applies to networking code across projects)

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
// Before implementing JWT auth

search_memory({
  query: "JWT token handling best practices and security",
  user_id: "sumankhadka",
  repo: "rust-mem",
  module: "src/auth",
  lang: "rust"
})

// Server returns memories ranked by semantic_score * boost:
// 1. Global JWT patterns (always surfaces)
// 2. Rust-specific implementations (lang boost)
// 3. Auth feature memories (feature boost)
// 4. rust-mem repo decisions (repo boost)
// 5. src/auth module details (biggest boost)
```

### Correct a Memory

```javascript
// User: "Actually, we use 15min tokens not 1hr"

// Find the memory ID from search results, then:

correct_memory({
  memory_id: "abc-123",
  user_id: "sumankhadka",
  correction: "Use 15min access tokens not 1hr — shorter window reduces replay risk",
  topics: ["auth", "jwt", "security", "tokens"]
})
```

## Planning Mode

When creating implementation plans:
1. Search memory for relevant patterns, preferences, and past decisions
2. Apply remembered coding style preferences to the plan
3. Reference any repo-specific conventions or architecture decisions
4. After plan is approved and implemented, save key learnings

## Quick Reference

```
🧠 Session start:
   get_memory_index → cache topics list

🧠 Before coding:
   search_memory(query, context for boost)

🧠 After solving:
   Check cached topics → reuse when applicable
   Pre-process → save_memory(fact, topics, scope, context)

🧠 User corrects:
   correct_memory(id, correction)

🧠 Pre-processing checklist:
   ✓ Extract clean fact (one sentence)
   ✓ Check cached topics from get_memory_index
   ✓ Reuse existing topics when applicable
   ✓ Determine 2-5 topics (create new only if needed)
   ✓ Decide scope (use decision tree)
   ✓ Pass context for scoping
```
