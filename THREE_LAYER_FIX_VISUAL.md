# The Three-Layer Fix (Visual Summary)

```
┌────────────────────────────────────────────────────────────────┐
│                     LAYER 1: JSON SCHEMA                       │
│                    (What's structurally valid)                 │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  {                                                             │
│    "required": ["content", "user_id", "scope"],               │
│    "properties": {                                             │
│      "scope": {                                                │
│        "type": "string",           ← Not nullable             │
│        "description": "Scope of the memory (REQUIRED):        │
│                       'global'|'lang'|'feature'|'repo'|       │
│                       'module'"    ← Valid values listed      │
│      },                                                        │
│      "repo": {                                                 │
│        "type": ["string", "null"], ← Nullable                 │
│        "default": null,            ← Has default              │
│        "description": "optional, but REQUIRED for             │
│                       'repo' and 'module' scopes"             │
│      }                                                         │
│    }                                                           │
│  }                                                             │
│                                                                │
│  ✅ Enforces: scope is required (in required array)           │
│  ✅ Enforces: scope must be string (not null)                 │
│  ✅ Documents: All valid scope values                         │
│  ✅ Explains: Conditional requirements                        │
│                                                                │
└────────────────────────────────────────────────────────────────┘
                              ▼
┌────────────────────────────────────────────────────────────────┐
│                  LAYER 2: TOOL DESCRIPTION                     │
│                 (What the values mean + usage)                 │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  "Save a new memory. Required: content, user_id, scope        │
│   (one of: global/lang/feature/repo/module).                  │
│                                                                │
│   Scope determines memory reach:                              │
│   - module (most specific, needs repo+module)                 │
│   - repo (needs repo)                                          │
│   - feature (needs feature)                                    │
│   - lang (needs lang)                                          │
│   - global (no extra fields)"                                  │
│                                                                │
│  ✅ Explains: What each scope value means                     │
│  ✅ Explains: What additional params each needs               │
│  ✅ Guides: Specificity ranking (module > repo > ... > global)│
│                                                                │
└────────────────────────────────────────────────────────────────┘
                              ▼
┌────────────────────────────────────────────────────────────────┐
│               LAYER 3: SYSTEM PROMPT + EXAMPLES                │
│                 (How to choose + patterns)                     │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  SCOPE DECISION TREE:                                          │
│  - "module": Memory specific to directory (requires: repo,    │
│              module path)                                      │
│  - "repo": Memory specific to repository (requires: repo)     │
│  - "feature": Memory about product feature (requires: feature)│
│  - "lang": Memory about programming language (requires: lang) │
│  - "global": General preference (no extra params)             │
│                                                                │
│  EXAMPLES:                                                     │
│  ├─ "Prefer Result over panic in Rust"                        │
│  │   → scope="lang", lang="rust"                              │
│  │                                                             │
│  ├─ "Quote links to Vendor in rust-mem"                       │
│  │   → scope="repo", repo="rust-mem"                          │
│  │                                                             │
│  ├─ "src/auth uses JWT with RS256"                            │
│  │   → scope="module", repo="myapp", module="src/auth"        │
│  │                                                             │
│  ├─ "Invoicing uses Stripe"                                   │
│  │   → scope="feature", feature="invoicing"                   │
│  │                                                             │
│  └─ "Prefer descriptive variable names"                       │
│      → scope="global"                                          │
│                                                                │
│  ✅ Teaches: Decision logic (how to choose)                   │
│  ✅ Shows: Concrete patterns to follow                        │
│  ✅ Demonstrates: Complete parameter sets                     │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

## How the Layers Work Together

```
User: "I prefer descriptive variable names"
         │
         ▼
┌─────────────────────────────────────────────────┐
│ Layer 1 (Schema) tells LLM:                    │
│ ✅ scope is required (in required array)        │
│ ✅ scope must be string (type validation)       │
│ ✅ valid values exist (see description)         │
└─────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────┐
│ Layer 2 (Tool Description) tells LLM:          │
│ ✅ scope="global" means universal preference    │
│ ✅ global scope needs no extra params           │
│ ✅ use this for preferences that apply anywhere │
└─────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────┐
│ Layer 3 (System Prompt) tells LLM:             │
│ ✅ "Prefer X" pattern matches global scope      │
│ ✅ Example: "Prefer descriptive names" → global │
│ ✅ No repo/lang/module needed for global        │
└─────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────┐
│ LLM generates tool call:                        │
│ save_memory(                                    │
│   content="Prefer descriptive variable names",  │
│   user_id="sumankhadka",                        │
│   scope="global",          ← ✅ Included!       │
│   topics=["code-style"]                         │
│ )                                               │
└─────────────────────────────────────────────────┘
         │
         ▼
      ✅ SUCCESS - Memory saved!
```

## Why You Need All Three

### Missing Layer 1 (Schema)
```
Problem: No structural enforcement
❌ LLM might omit scope even with instructions
❌ Deserialization fails if LLM makes mistake
❌ No validation of parameter types
```

### Missing Layer 2 (Tool Description)
```
Problem: No semantic guidance
❌ LLM knows scope is required but not what values are valid
❌ LLM doesn't know what each scope means
❌ LLM doesn't know dependencies (repo needed for repo scope)
```

### Missing Layer 3 (System Prompt + Examples)
```
Problem: No decision logic
❌ LLM knows valid values but not how to choose
❌ No patterns to follow for different scenarios
❌ Harder for LLM to apply abstract rules
```

## The Complete Fix

```
Layer 1 (Schema)      →  "scope is required and must be string"
         +
Layer 2 (Description) →  "scope values: global/lang/feature/repo/module"
         +
Layer 3 (Examples)    →  "prefer X → global, Y in Rust → lang"
         ║
         ║
         ▼
    ✅ LLM includes scope with correct value every time!
```

## Files Changed

```
mcp/src/tools.rs                     ← Layer 1: Schema annotations
mcp/src/agent/tools.rs               ← Layer 2: Tool descriptions
mcp/src/agent/agents/memory_chat.rs  ← Layer 3: System prompt + examples
mcp/src/main.rs                      ← Layer 2: MCP tool description
```

## Verification

```bash
# Check Layer 1 (Schema)
cd mcp && cargo run --example test_schema | jq '.required'
# Should output: ["content", "user_id", "scope"]

# Check Layer 2 (Tool description in agent)
rg "scope.*one of" mcp/src/agent/tools.rs

# Check Layer 3 (Examples in system prompt)
rg "EXAMPLES:" mcp/src/agent/agents/memory_chat.rs -A 5
```

All three layers must be present for the fix to work reliably!
