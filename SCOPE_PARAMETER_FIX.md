# Fix: Agent Missing Required `scope` Parameter

## Problem

The memory chat agent was consistently failing on first `save_memory` calls with:
```
{"error":"MCP error -32602: failed to deserialize parameters: missing field `scope`"}
```

## Root Cause

The agent's system prompt was not providing clear enough guidance about the **required** `scope` parameter:

1. **Tool description was vague**: Just said "Required: content, user_id, scope, topics" without explaining what `scope` should be
2. **No decision tree provided**: The MCP tool description mentioned a "decision tree" but didn't include it in the agent's instructions
3. **LLM confusion**: Without clear guidance, the LLM would sometimes omit `scope` or not know what value to use

## Solution

### 1. Enhanced System Prompt (`memory_chat.rs`)

Added explicit scope decision tree with examples:

```rust
SCOPE DECISION TREE (pick the most specific that applies):
- "module": Memory specific to a directory/module (requires: repo, module path)
- "repo": Memory specific to a repository (requires: repo)
- "feature": Memory about a product feature across repos (requires: feature name)
- "lang": Memory about a programming language/tech (requires: lang)
- "global": General preference or pattern that applies everywhere

EXAMPLES:
- User prefers Result over panic in Rust → scope="lang", lang="rust"
- Quote links to Vendor in rust-mem repo → scope="repo", repo="rust-mem"
- Auth module uses JWT → scope="module", repo="myapp", module="src/auth"
- Invoicing feature uses Stripe → scope="feature", feature="invoicing"
- User prefers descriptive variable names → scope="global"
```

### 2. Improved Tool Description (`tools.rs`)

Made the tool description more explicit about requirements:

```rust
"Save a new memory. Required: content, user_id, scope (one of: global/lang/feature/repo/module). 
Optional: topics, repo, lang, module, feature. Scope determines memory reach: module (most 
specific, needs repo+module), repo (needs repo), feature (needs feature), lang (needs lang), 
global (no extra fields). Automatically handles deduplication and contradiction detection."
```

### 3. Enhanced Parameter Schema (`tools.rs`)

Updated the `scope` parameter description in the JSON schema to be crystal clear:

```json
"scope": {
    "type": "string",
    "enum": ["global", "lang", "feature", "repo", "module"],
    "description": "REQUIRED. Scope of the memory: 'global' (universal), 'lang' (language-specific, 
    requires lang param), 'feature' (product feature, requires feature param), 'repo' 
    (repository-specific, requires repo param), 'module' (module/directory, requires repo+module params)"
}
```

## Why This Fixes It

1. **Clear requirements**: The agent now knows `scope` is REQUIRED (spelled out in caps)
2. **Decision tree**: The agent has a systematic way to choose the right scope value
3. **Examples**: Concrete examples help the LLM understand the patterns
4. **Explicit enum values**: The agent can see all valid scope values in both the system prompt and the schema
5. **Dependency guidance**: The agent knows which additional fields are needed for each scope type

## Files Changed

- `mcp/src/agent/agents/memory_chat.rs` - Enhanced system prompt with scope decision tree
- `mcp/src/agent/tools.rs` - Improved tool description and parameter schema

## Testing

After rebuilding the agent:
```bash
cd mcp
cargo build --bin memory_chat_agent
```

The agent should now:
1. Always include `scope` in `save_memory` calls
2. Choose appropriate scope values based on the memory content
3. Include required additional parameters (repo, lang, module, feature) based on scope

## Prevention

To prevent similar issues in the future:
1. **Always document required parameters clearly** in system prompts
2. **Provide decision trees or algorithms** when parameters need logic to determine
3. **Include examples** for complex parameter combinations
4. **Use "REQUIRED" in caps** in parameter descriptions to emphasize criticality
5. **Test with real LLMs** to ensure instructions are clear enough
