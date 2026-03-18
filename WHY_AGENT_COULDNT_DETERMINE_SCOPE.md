# Why Agent Couldn't Determine Required `scope` Parameter from MCP

## The Issue

The memory chat agent was consistently failing on first `save_memory` attempts with:
```
{"error":"MCP error -32602: failed to deserialize parameters: missing field `scope`"}
```

## Root Cause Analysis

The agent **couldn't** determine that `scope` was required and how to use it because of **insufficient documentation in the tool definitions**. Here's why:

### 1. MCP Tool Schema Said Required, But Not How

The MCP tool JSON (`save_memory.json`) had:
```json
"required": ["content", "user_id", "scope"]
```

But the description just said:
```
"Decide scope using decision tree (global/lang/feature/repo/module)"
```

**Problem**: Mentioned a "decision tree" but didn't include it!

### 2. Agent System Prompt Was Vague

The agent's internal system prompt (`memory_chat.rs`) said:
```
save_memory: Save new information. Required: content, user_id, scope, topics.
```

**Problem**: Listed requirements but gave zero guidance on:
- What valid scope values are
- How to choose between them
- What additional params are needed for each scope

### 3. LLM Behavior Without Clear Instructions

When an LLM sees:
- A required parameter with no clear valid values
- No examples showing how to use it
- No decision logic for choosing values

It will either:
1. **Omit the parameter** (hoping it's actually optional)
2. **Guess a value** (often wrong)
3. **Ask the user** (breaking the flow)

In this case, the LLM chose option 1 - omitting `scope` entirely, causing the deserialization error.

## The Fix

We added **explicit, actionable instructions** in three places:

### 1. Enhanced Agent System Prompt

Added a complete decision tree with examples:

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

### 2. Improved Agent Tool Description

Made it crystal clear in the tool metadata:

```rust
"Save a new memory. Required: content, user_id, scope (one of: global/lang/feature/repo/module). 
Optional: topics, repo, lang, module, feature. Scope determines memory reach: module (most 
specific, needs repo+module), repo (needs repo), feature (needs feature), lang (needs lang), 
global (no extra fields)."
```

### 3. Enhanced MCP Server Tool Description

Updated the main MCP server tool definition to include the decision logic inline:

```rust
"(3) Decide scope using this decision tree: 'global'=universal preference, 
'lang'=language-specific (needs lang param), 'feature'=product feature across repos 
(needs feature param), 'repo'=repository-specific (needs repo param), 
'module'=directory-specific (needs repo+module params). Choose most specific scope that applies."
```

### 4. Explicit Parameter Descriptions

Enhanced the JSON schema parameter descriptions:

```rust
"scope": {
    "type": "string",
    "enum": ["global", "lang", "feature", "repo", "module"],
    "description": "REQUIRED. Scope of the memory: 'global' (universal), 'lang' 
    (language-specific, requires lang param), 'feature' (product feature, requires feature param), 
    'repo' (repository-specific, requires repo param), 'module' (module/directory, requires 
    repo+module params)"
}
```

## Why This Works

The fix works because we provided:

1. **Clear enumeration**: All 5 valid scope values explicitly listed
2. **Decision logic**: A systematic way to choose the right value
3. **Dependency requirements**: What extra params are needed for each scope
4. **Concrete examples**: Real-world scenarios showing how to apply the logic
5. **Emphasis on requirement**: "REQUIRED" in caps to signal criticality

## Lessons for LLM Tool Design

When designing tools for LLMs to call:

### ✅ DO:
- **List all valid enum values** explicitly in descriptions
- **Provide decision trees or flowcharts** for choosing between options
- **Include 3-5 concrete examples** showing parameter combinations
- **Mark truly required params** with "REQUIRED" in caps
- **Explain dependencies** between parameters (e.g., "requires repo param")
- **Test with real LLMs** to verify instructions are clear enough

### ❌ DON'T:
- Assume the LLM "knows" common values (it doesn't without context)
- Reference external docs ("see decision tree") without including them
- Use vague descriptions ("decide scope appropriately")
- Leave parameter relationships implicit
- Skip examples for complex parameter combinations

## Why LLMs Need This Level of Detail

Unlike humans who can:
- Infer context from domain knowledge
- Ask clarifying questions easily
- Reference external documentation
- Learn from trial and error

LLMs need:
- **Everything in the prompt**: They can't look things up
- **Explicit instructions**: They can't infer unstated rules
- **Examples over theory**: They pattern-match more than reason
- **Clear validation rules**: They need to know what's valid before calling

## Files Changed

1. `mcp/src/agent/agents/memory_chat.rs` - Agent system prompt with decision tree
2. `mcp/src/agent/tools.rs` - Agent tool descriptions and parameter schemas
3. `mcp/src/main.rs` - MCP server tool description with inline decision logic

## Testing

Run the test script:
```bash
cd mcp
./test_scope_fix.sh
```

Or test manually:
```bash
cd mcp
cargo run --bin memory_chat_agent
```

Try these test cases:
1. **Global**: "I prefer descriptive variable names" → should use `scope="global"`
2. **Lang**: "In Rust, prefer Result over panic" → should use `scope="lang", lang="rust"`
3. **Repo**: "Quote links to Vendor in rust-mem" → should use `scope="repo", repo="rust-mem"`

Verify:
- ✅ No "missing field scope" errors
- ✅ Agent chooses appropriate scope values
- ✅ Agent includes required additional parameters

## Impact

Before fix:
- ❌ First save_memory call always failed
- ❌ Agent had to retry after error
- ❌ Poor user experience
- ❌ Wasted API tokens on retries

After fix:
- ✅ First save_memory call succeeds
- ✅ Agent chooses correct scope immediately
- ✅ Smooth user experience
- ✅ No wasted API calls
